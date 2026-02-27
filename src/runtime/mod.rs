use alloy::primitives::Address;
use alloy_sol_types::{Eip712Domain, eip712_domain};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::error::EtherealRuntimeError;
use crate::executor::{LiveExecutor, OrderExecutorRuntime, PaperExecutor};
use crate::logging::targets;
use crate::models::common::TimeInForce;
use crate::models::contracts::TradeOrder;
use crate::models::dto::{
    CancelOrderData, CancelOrderRequest, MarketPriceData, OrderRequest, OrderUpdateData, Timestamp,
    TradeOrderData, WsEvent, parse_ws_event,
};
use crate::settings::{Config, ExecutionMode};

#[derive(Debug, Clone)]
pub enum RuntimeEvent {
    OrderUpdate(OrderUpdateData),
    MarketPrice(MarketPriceData),
}

type WsWriteType = futures_util::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

type WsReadType = futures_util::stream::SplitStream<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

fn build_subscribe_order_updates_frame(subaccount_id: Uuid) -> String {
    format!(
        r#"42/v1/stream,["subscribe",{{"type":"OrderUpdate","subaccountId":"{subaccount_id}"}}]"#
    )
}

fn build_subscribe_market_price_frame(product_id: Uuid) -> String {
    format!(r#"42/v1/stream,["subscribe",{{"type":"MarketPrice","productId":"{product_id}"}}]"#)
}

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
    order_executor: OrderExecutorRuntime,

    ws_sender: mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
}

impl EtherealRuntime {
    pub async fn new(
        config: &Config,
    ) -> Result<(Self, mpsc::UnboundedReceiver<RuntimeEvent>), EtherealRuntimeError> {
        let (ws_write, ws_read) = Self::connect_ws(&config.ws_url).await?;
        let (ws_sender, ws_receiver) = tokio::sync::mpsc::channel(32);
        let (event_sender, event_receiver) = mpsc::unbounded_channel();

        let http_client = reqwest::Client::new();
        let order_executor = match config.execution_mode {
            ExecutionMode::Live => {
                OrderExecutorRuntime::Live(LiveExecutor::new(http_client, config.rest_url.clone()))
            }
            ExecutionMode::Paper => OrderExecutorRuntime::Paper(PaperExecutor::new(
                http_client,
                config.rest_url.clone(),
            )),
        };

        tokio::spawn(Self::spawn_write_job(ws_write, ws_receiver));
        tokio::spawn(Self::spawn_read_job(
            ws_read,
            ws_sender.clone(),
            event_sender,
        ));

        Ok((
            Self {
                signer: crate::signer::Signer::new(&config.signer_config),
                domain: make_domain(config.chain_id, config.exchange),
                order_executor,
                ws_sender,
            },
            event_receiver,
        ))
    }

    async fn connect_ws(
        ws_url: &url::Url,
    ) -> Result<(WsWriteType, WsReadType), EtherealRuntimeError> {
        use tokio_tungstenite::tungstenite::Message;

        tracing::info!(
            target: targets::RUNTIME_WS,
            %ws_url,
            "connecting websocket"
        );

        let url = ws_url.join("socket.io/?EIO=4&transport=websocket")?;

        let (ws_stream, response) = tokio_tungstenite::connect_async(url.to_string()).await?;

        if response.status() != reqwest::StatusCode::SWITCHING_PROTOCOLS {
            return Err(EtherealRuntimeError::WS(format!(
                "unexpected status: {}",
                response.status(),
            )));
        }

        let (mut write, mut read) = ws_stream.split();

        read.next().await;

        write.send(Message::Text("40/v1/stream,".into())).await?;
        read.next().await;

        tracing::info!(
            target: targets::RUNTIME_WS,
            "websocket namespace connected"
        );

        Ok((write, read))
    }

    async fn spawn_write_job(
        mut ws_write: WsWriteType,
        mut ws_receiver: tokio::sync::mpsc::Receiver<tokio_tungstenite::tungstenite::Message>,
    ) {
        while let Some(msg) = ws_receiver.recv().await {
            if let Err(error) = ws_write.send(msg).await {
                tracing::error!(
                    target: targets::RUNTIME_WS,
                    %error,
                    "websocket write failed"
                );
            }
        }
    }

    async fn spawn_read_job(
        mut ws_read: WsReadType,
        ws_sender: mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
        event_sender: mpsc::UnboundedSender<RuntimeEvent>,
    ) {
        use tokio_tungstenite::tungstenite::Message;

        loop {
            match ws_read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if text == "2" {
                        if let Err(error) = ws_sender.send(Message::Text("3".into())).await {
                            tracing::warn!(
                                target: targets::RUNTIME_WS,
                                %error,
                                "failed to enqueue websocket pong"
                            );
                        }
                        continue;
                    }

                    if let Some(event) = parse_ws_event(&text) {
                        match event {
                            WsEvent::OrderUpdate(updates) => {
                                for update in updates {
                                    event_sender
                                        .send(RuntimeEvent::OrderUpdate(update))
                                        .expect("runtime event receiver dropped");
                                }
                            }
                            WsEvent::MarketPrice(prices) => {
                                for price in prices {
                                    event_sender
                                        .send(RuntimeEvent::MarketPrice(price))
                                        .expect("runtime event receiver dropped");
                                }
                            }
                            WsEvent::Unknown { event, payload } => {
                                tracing::debug!(
                                    target: targets::RUNTIME_WS,
                                    event,
                                    %payload,
                                    "received unknown websocket event"
                                );
                            }
                        }
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(error)) => {
                    tracing::error!(
                        target: targets::RUNTIME_WS,
                        %error,
                        "websocket read error"
                    );
                    break;
                }
                None => break,
            }
        }
    }

    pub async fn subscribe_order_updates(
        &self,
        subaccount_id: Uuid,
    ) -> Result<(), EtherealRuntimeError> {
        use tokio_tungstenite::tungstenite::Message;

        let msg = build_subscribe_order_updates_frame(subaccount_id);

        self.ws_sender
            .send(Message::Text(msg.into()))
            .await
            .expect("websocket sender dropped");

        tracing::info!(
            target: targets::RUNTIME_WS,
            %subaccount_id,
            "subscribed to order updates"
        );

        Ok(())
    }

    pub async fn subscribe_market_price(
        &self,
        product_id: Uuid,
    ) -> Result<(), EtherealRuntimeError> {
        use tokio_tungstenite::tungstenite::Message;

        let msg = build_subscribe_market_price_frame(product_id);

        self.ws_sender
            .send(Message::Text(msg.into()))
            .await
            .expect("websocket sender dropped");

        tracing::info!(
            target: targets::RUNTIME_WS,
            %product_id,
            "subscribed to market price"
        );

        Ok(())
    }

    pub async fn place_order(
        &self,
        price_raw: u128,
        qty_raw: u128,
        side: u8,
        product_id: u32,
        post_only: bool,
        time_in_force: TimeInForce,
    ) -> Result<Uuid, EtherealRuntimeError> {
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

        tracing::info!(
            target: targets::RUNTIME_EXEC,
            %client_order_id,
            product_id,
            side,
            "submitting order"
        );

        let order = OrderRequest {
            data,
            signature: format!("0x{}", hex::encode(signature.as_bytes())),
        };

        let payload = match self.order_executor.submit_order(&order).await {
            Ok(value) => value,
            Err(error) => {
                tracing::warn!(
                    target: targets::RUNTIME_EXEC,
                    %client_order_id,
                    %error,
                    "order submission failed"
                );
                return Err(error.into());
            }
        };

        tracing::info!(
            target: targets::RUNTIME_EXEC,
            %client_order_id,
            %payload,
            "order accepted"
        );

        Ok(client_order_id)
    }

    pub async fn cancel_order(&self, client_order_id: Uuid) -> Result<(), EtherealRuntimeError> {
        let ts_cancel = Timestamp::now();
        let (cancel_sig, order) = self.signer.sign_cancel_order(ts_cancel.nonce, &self.domain);
        let cancel_data = CancelOrderData::from_cancel_order(order, vec![], vec![client_order_id]);
        let cancel_req = CancelOrderRequest {
            data: cancel_data,
            signature: format!("0x{}", hex::encode(cancel_sig.as_bytes())),
        };

        tracing::info!(
            target: targets::RUNTIME_EXEC,
            %client_order_id,
            "submitting cancel"
        );

        self.order_executor.cancel_order(&cancel_req).await?;
        tracing::info!(
            target: targets::RUNTIME_EXEC,
            %client_order_id,
            "cancel accepted"
        );
        Ok(())
    }

    pub async fn _shutdown(&mut self) -> Result<(), EtherealRuntimeError> {
        todo!("graceful runtime shutdown is not implemented yet")
    }
}

#[cfg(test)]
mod ws_subscription_tests {
    use uuid::Uuid;

    use super::{build_subscribe_market_price_frame, build_subscribe_order_updates_frame};
    use crate::EtherealRuntimeError;

    #[test]
    fn order_update_subscribe_frame_is_correct() {
        let id = Uuid::new_v4();

        let frame = build_subscribe_order_updates_frame(id);
        assert_eq!(
            frame,
            format!(r#"42/v1/stream,["subscribe",{{"type":"OrderUpdate","subaccountId":"{id}"}}]"#)
        );
    }

    #[test]
    fn market_price_subscribe_frame_is_correct() {
        let product_id = Uuid::parse_str("bc7d5575-3711-4532-a000-312bfacfb767").unwrap();
        let frame = build_subscribe_market_price_frame(product_id);
        assert_eq!(
            frame,
            r#"42/v1/stream,["subscribe",{"type":"MarketPrice","productId":"bc7d5575-3711-4532-a000-312bfacfb767"}]"#
        );
    }

    #[test]
    fn tungstenite_error_maps_to_ws_string() {
        let err: EtherealRuntimeError =
            tokio_tungstenite::tungstenite::Error::ConnectionClosed.into();

        match err {
            EtherealRuntimeError::WS(message) => {
                assert!(message.to_lowercase().contains("closed"));
            }
            other => panic!("unexpected error variant: {other:?}"),
        }
    }
}
