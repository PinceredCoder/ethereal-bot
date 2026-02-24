mod backend;
mod error;
mod models;
mod settings;
mod signer;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use alloy::primitives::Address;
use alloy_sol_types::{Eip712Domain, eip712_domain};
use dashmap::DashMap;
pub use error::EtherealRuntimeError;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot};
use tokio::time::Instant;
use uuid::Uuid;

use crate::backend::{LiveBackend, OrderBackend};
use crate::models::contracts::TradeOrder;
use crate::models::dto::{
    CancelOrderData, CancelOrderRequest, CancelOrderResult, OrderRequest, OrderUpdateData,
    SubmitOrderResult, Timestamp, TradeOrderData, parse_order_update,
};
use crate::settings::{Config, ExecutionMode};

type WsWriteType = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

type WsReadType = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

pub(crate) fn make_domain(chain_id: u64, exchange: Address) -> Eip712Domain {
    eip712_domain! {
        name: "Ethereal",
        version: "1",
        chain_id: chain_id,
        verifying_contract: exchange,
    }
}

pub struct EtherealRuntime {
    signer: crate::signer::Signer,
    domain: Eip712Domain,
    // TODO(step-4+): evaluate replacing dyn dispatch with compile-time backend wiring.
    order_backend: Arc<dyn OrderBackend>,

    ws_sender: mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
    pending_orders: Arc<DashMap<Uuid, oneshot::Sender<(OrderUpdateData, Instant)>>>,
}

impl EtherealRuntime {
    pub async fn new(config: &Config) -> Result<Self, EtherealRuntimeError> {
        let (ws_write, ws_read) = Self::connect_ws(&config.ws_url).await?;
        let (ws_sender, ws_receiver) = tokio::sync::mpsc::channel(32);

        let pending_orders = Arc::new(DashMap::new());
        let http_client = reqwest::Client::new();
        let order_backend: Arc<dyn OrderBackend> = match config.execution_mode {
            ExecutionMode::Live => Arc::new(LiveBackend::new(http_client, config.rest_url.clone())),
            ExecutionMode::Paper => {
                return Err(EtherealRuntimeError::ExecutionModeNotImplemented("paper"));
            }
        };

        tokio::spawn(Self::spawn_write_job(ws_write, ws_receiver));
        tokio::spawn(Self::spawn_read_job(
            ws_read,
            ws_sender.clone(),
            Arc::clone(&pending_orders),
        ));

        Ok(Self {
            signer: crate::signer::Signer::new(&config.signer_config),
            domain: make_domain(config.chain_id, config.exchange),
            order_backend,
            ws_sender,
            pending_orders,
        })
    }

    async fn connect_ws(
        ws_url: &url::Url,
    ) -> Result<(WsWriteType, WsReadType), EtherealRuntimeError> {
        use tokio_tungstenite::tungstenite::Message;

        let url = ws_url.join("socket.io/?EIO=4&transport=websocket")?;

        let (ws_stream, response) = tokio_tungstenite::connect_async(url.to_string()).await?;

        if response.status() != reqwest::StatusCode::SWITCHING_PROTOCOLS {
            return Err(EtherealRuntimeError::Connection(format!(
                "unexpected status: {}",
                response.status(),
            )));
        }

        let (mut write, mut read) = ws_stream.split();

        // Handshake
        read.next().await;

        // Connect namespace
        write.send(Message::Text("40/v1/stream,".into())).await?;
        read.next().await;

        Ok((write, read))
    }

    async fn spawn_write_job(
        mut ws_write: WsWriteType,
        mut ws_receiver: tokio::sync::mpsc::Receiver<tokio_tungstenite::tungstenite::Message>,
    ) {
        while let Some(msg) = ws_receiver.recv().await {
            let mut attempts = 0u32;
            loop {
                match ws_write.send(msg.clone()).await {
                    Ok(_) => break,
                    Err(e) => {
                        attempts += 1;
                        if attempts >= 3 {
                            eprintln!("WS write failed after {attempts} attempts: {e}");
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(
                            100 * 2u64.pow(attempts),
                        ))
                        .await;
                    }
                }
            }
        }
    }

    async fn spawn_read_job(
        mut ws_read: WsReadType,
        ws_sender: mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
        pending_orders: Arc<DashMap<Uuid, oneshot::Sender<(OrderUpdateData, Instant)>>>,
    ) {
        use tokio_tungstenite::tungstenite::Message;

        loop {
            match ws_read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if text == "2" {
                        let _ = ws_sender.send(Message::Text("3".into())).await;
                        continue;
                    }

                    let received_at = Instant::now();

                    if let Some(payload) = parse_order_update(&text) {
                        for update in payload.data {
                            if let Some((_, sender)) =
                                pending_orders.remove(&update.client_order_id)
                            {
                                let _ = sender.send((update, received_at));
                            }
                        }
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => {
                    eprintln!("WS read error: {e}");
                    break;
                }
                None => break,
            }
        }
    }

    pub async fn subscribe_order_updates(
        &self,
        subaccount_id: &str,
    ) -> Result<(), EtherealRuntimeError> {
        use tokio_tungstenite::tungstenite::Message;

        let msg = format!(
            r#"42/v1/stream,["subscribe",{{"type":"OrderUpdate","subaccountId":"{subaccount_id}"}}]"#
        );

        self.ws_sender
            .send(Message::Text(msg.into()))
            .await
            .expect("receiver is dropped");

        Ok(())
    }

    pub async fn place_order(
        &self,
        price_raw: u128,
        qty_raw: u128,
        side: u8,
        product_id: u32,
        post_only: bool,
        time_in_force: &'static str,
    ) -> Result<
        (
            Uuid,
            tokio::time::Instant,
            oneshot::Receiver<(OrderUpdateData, Instant)>,
        ),
        EtherealRuntimeError,
    > {
        let ts = Timestamp::now();

        let order = TradeOrder {
            sender: *self.signer.accound_address(),
            subaccount: *self.signer.subaccount(),
            price: price_raw,
            quantity: qty_raw,
            side,
            engineType: 0,
            productId: product_id,
            nonce: ts.nonce,
            signedAt: ts.signed_at,
            reduceOnly: false,
        };

        let signature = self.signer.sign_trade_order(&order, &self.domain);
        let data = TradeOrderData::from_trade_order(order, post_only, time_in_force);

        let client_order_id = data.client_order_id;

        let order = OrderRequest {
            data,
            signature: format!("0x{}", hex::encode(signature.as_bytes())),
        };

        // Регистрируем до отправки
        let (sender, receiver) = oneshot::channel();
        self.pending_orders.insert(client_order_id, sender);

        let t0 = tokio::time::Instant::now();
        let submit_result = self.order_backend.submit_order(&order).await;
        let submit_result = match submit_result {
            Ok(value) => value,
            Err(err) => {
                self.pending_orders.remove(&client_order_id);
                return Err(err);
            }
        };

        if let SubmitOrderResult::Rejected { payload } = submit_result {
            self.pending_orders.remove(&client_order_id);
            return Err(EtherealRuntimeError::OrderRejected(payload.to_string()));
        }

        Ok((client_order_id, t0, receiver))
    }

    pub async fn cancel_order(&mut self, order_id: Uuid) -> Result<(), EtherealRuntimeError> {
        let ts_cancel = Timestamp::now();
        let (cancel_sig, order) = self.signer.sign_cancel_order(ts_cancel.nonce, &self.domain);
        let cancel_data = CancelOrderData::from_cancel_order(order, vec![order_id], vec![]);
        let cancel_req = CancelOrderRequest {
            data: cancel_data,
            signature: format!("0x{}", hex::encode(cancel_sig.as_bytes())),
        };

        match self.order_backend.cancel_order(&cancel_req).await? {
            CancelOrderResult::Accepted { .. } => Ok(()),
            CancelOrderResult::Rejected { payload } => {
                Err(EtherealRuntimeError::CancelRejected(payload.to_string()))
            }
        }
    }
}
