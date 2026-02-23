mod error;
mod models;
mod settings;
mod signer;

#[cfg(test)]
mod tests;

use alloy::primitives::Address;
use alloy_sol_types::{Eip712Domain, eip712_domain};
pub use error::EtherealBotError;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use uuid::Uuid;

use crate::models::{
    CancelOrderData, CancelOrderRequest, OrderRequest, Timestamp, TradeOrder, TradeOrderData,
};
use crate::settings::Config;

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

pub struct EtherealBot {
    signer: crate::signer::Signer,
    http_client: reqwest::Client,
    domain: Eip712Domain,

    rest_url: url::Url,

    ws_read: WsReadType,
    ws_write: WsWriteType,
}

impl EtherealBot {
    pub async fn new(config: &Config) -> Result<Self, EtherealBotError> {
        let (ws_write, ws_read) = Self::connect_ws(&config.ws_url).await?;

        Ok(Self {
            signer: crate::signer::Signer::new(&config.signer_config),
            domain: make_domain(config.chain_id, config.exchange.parse()?),
            http_client: reqwest::Client::new(),
            rest_url: config.rest_url.clone(),
            ws_read,
            ws_write,
        })
    }

    async fn connect_ws(ws_url: &url::Url) -> Result<(WsWriteType, WsReadType), EtherealBotError> {
        use tokio_tungstenite::tungstenite::Message;

        let url = ws_url.join("socket.io/?EIO=4&transport=websocket")?;

        let (ws_stream, response) = tokio_tungstenite::connect_async(url.to_string()).await?;

        if response.status() != reqwest::StatusCode::SWITCHING_PROTOCOLS {
            return Err(EtherealBotError::Connection(format!(
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

    pub async fn subscribe_order_updates(
        &mut self,
        subaccount_id: &str,
    ) -> Result<(), EtherealBotError> {
        use tokio_tungstenite::tungstenite::Message;

        let subscribe_message = format!(
            r#"42/v1/stream,["subscribe",{{"type":"OrderUpdate","subaccountId":"{subaccount_id}"}}]"#
        );

        self.ws_write
            .send(Message::Text(subscribe_message.into()))
            .await?;

        Ok(())
    }

    pub async fn next_ws_message(&mut self) -> Result<Option<String>, EtherealBotError> {
        use tokio_tungstenite::tungstenite::Message;

        loop {
            match self.ws_read.next().await {
                Some(Ok(Message::Text(text))) => {
                    if text == "2" {
                        // Engine.IO ping -> pong
                        self.ws_write.send(Message::Text("3".into())).await?;
                    } else {
                        return Ok(Some(text.to_string()));
                    }
                }
                Some(Ok(_)) => continue,
                Some(Err(e)) => return Err(EtherealBotError::WS(e)),
                None => return Ok(None),
            }
        }
    }

    pub async fn place_order(
        &self,
        price_raw: u128,
        qty_raw: u128,
        side: u8,
        product_id: u32,
        post_only: bool,
        time_in_force: &'static str,
    ) -> Result<(Uuid, std::time::Instant), EtherealBotError> {
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
        let order = OrderRequest {
            data,
            signature: format!("0x{}", hex::encode(signature.as_bytes())),
        };

        let t0 = std::time::Instant::now();
        let res = self
            .http_client
            .post(format!("{}/v1/order", self.rest_url))
            .json(&order)
            .send()
            .await?;

        let body: serde_json::Value = res.json().await?;
        let order_id = Uuid::deserialize(
            body.get("id")
                .ok_or_else(|| EtherealBotError::Api("missing id".to_string()))?,
        )
        .map_err(|e| EtherealBotError::Api(format!("bad uuid format: {e}")))?;

        Ok((order_id, t0))
    }

    pub async fn cancel_order(&mut self, order_id: Uuid) -> Result<(), EtherealBotError> {
        let ts_cancel = crate::models::Timestamp::now();
        let (cancel_sig, order) = self.signer.sign_cancel_order(ts_cancel.nonce, &self.domain);
        let cancel_data = CancelOrderData::from_cancel_order(order, vec![order_id], vec![]);
        let cancel_req = CancelOrderRequest {
            data: cancel_data,
            signature: format!("0x{}", hex::encode(cancel_sig.as_bytes())),
        };

        self.http_client
            .post(format!("{}/v1/order/cancel", self.rest_url))
            .json(&cancel_req)
            .send()
            .await?;

        Ok(())
    }
}

// #[cfg(test)]
// mod tests_d {
//     use std::str::FromStr;

//     use alloy::network::EthereumWallet;
//     use alloy::primitives::{Address, FixedBytes, U256};
//     use alloy::providers::{Provider, ProviderBuilder};
//     use alloy::signers::local::PrivateKeySigner;
//     use serde_json::json;

//     use crate::models::{
//         CancelOrderData, CancelOrderRequest, OrderRequest, TradeOrder, TradeOrderData,
//     };
//     use crate::{EXCHANGE, IERC20, IExchange, IWUSDE, RPC_URL, TOKEN};

//     #[tokio::test]
//     async fn dry_run_post_only() {
//         let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY env var required");

//         let config = crate::settings::Config::testnet(private_key);
//         let signer = crate::signer::Signer::new(&config.signer_config);
//         let ts = crate::models::Timestamp::now();

//         let price = 50_000 * 1_000_000_000;
//         let quantity = 1_000_000_000 / 10_000;
//         let product_id: u32 = 1; // BTCUSD

//         let order = TradeOrder {
//             sender: *signer.accound_address(),
//             subaccount: *signer.subaccount(),
//             price,
//             quantity,
//             side: 0,
//             engineType: 0,
//             productId: product_id,
//             nonce: ts.nonce,
//             signedAt: ts.signed_at,
//             reduceOnly: false,
//         };

//         let data = TradeOrderData::from_trade_order(order, true, "GTD".into());

//         let client = reqwest::Client::new();
//         let res = client
//             .post(format!("{}/v1/order/dry-run", config.rest_url))
//             .json(&json!({
//                 "data": data
//             }))
//             .send()
//             .await
//             .unwrap();

//         println!("status: {}", res.status());
//         println!("body: {}", res.text().await.unwrap());
//     }

//     #[tokio::test]
//     #[ignore]
//     async fn measure_post_only_latency() {
//         let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY env var required");

//         let config = crate::settings::Config::testnet(private_key);
//         let signer = crate::signer::Signer::new(&config.signer_config);
//         let domain = crate::make_domain(config.chain_id, config.exchange.parse().unwrap());

//         let price: u128 = 50_000 * 1_000_000_000;
//         let quantity: u128 = 1_000_000_000 / 1000;
//         let product_id: u32 = 1; // BTCUSD

//         let client = reqwest::Client::new();

//         for i in 0..5 {
//             let ts = crate::models::Timestamp::now();

//             let order = TradeOrder {
//                 sender: *signer.accound_address(),
//                 subaccount: *signer.subaccount(),
//                 price,
//                 quantity,
//                 side: 0,
//                 engineType: 0,
//                 productId: product_id,
//                 nonce: ts.nonce,
//                 signedAt: ts.signed_at,
//                 reduceOnly: false,
//             };

//             let signature = signer.sign_trade_order(&order, &domain);

//             let order = OrderRequest {
//                 data: TradeOrderData::from_trade_order(order, true, "GTD".into()),
//                 signature: format!("0x{}", hex::encode(signature.as_bytes())),
//             };

//             let t0 = std::time::Instant::now();
//             let res = client
//                 .post(format!("{}/v1/order", config.rest_url))
//                 .json(&order)
//                 .send()
//                 .await
//                 .unwrap();
//             let rtt = t0.elapsed();

//             let status = res.status();
//             let body: serde_json::Value = res.json().await.unwrap();
//             let order_id = body["id"].as_str().unwrap().parse().unwrap();

//             println!("#{i} status: {status}, RTT: {rtt:?}");
//             println!("#{i} body: {body}");

//             // Отменяем ордер
//             let ts_cancel = crate::models::Timestamp::now();
//             let (cancel_sig, order) = signer.sign_cancel_order(ts_cancel.nonce, &domain);
//             let cancel_data = CancelOrderData::from_cancel_order(order, vec![order_id], vec![]);
//             let cancel_req = CancelOrderRequest {
//                 data: cancel_data,
//                 signature: format!("0x{}", hex::encode(cancel_sig.as_bytes())),
//             };

//             let cancel_res = client
//                 .post(format!("{}/v1/order/cancel", config.rest_url))
//                 .json(&cancel_req)
//                 .send()
//                 .await
//                 .unwrap();
//             println!("#{i} cancel: {}", cancel_res.text().await.unwrap());
//         }
//     }

//     #[tokio::test]
//     #[ignore]
//     async fn measure_ioc_latency() {
//         let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY env var required");

//         let config = crate::settings::Config::testnet(private_key);
//         let signer = crate::signer::Signer::new(&config.signer_config);
//         let domain = crate::make_domain(config.chain_id, config.exchange.parse().unwrap());

//         let price: u128 = 100_000 * 1_000_000_000;
//         let quantity: u128 = 1_000_000_000 / 10_000;
//         let product_id: u32 = 1; // BTCUSD

//         let client = reqwest::Client::new();

//         for i in 0..5 {
//             let ts = crate::models::Timestamp::now();

//             let order = TradeOrder {
//                 sender: *signer.accound_address(),
//                 subaccount: *signer.subaccount(),
//                 price,
//                 quantity,
//                 side: 0,
//                 engineType: 0,
//                 productId: product_id,
//                 nonce: ts.nonce,
//                 signedAt: ts.signed_at,
//                 reduceOnly: false,
//             };

//             let signature = signer.sign_trade_order(&order, &domain);

//             let order = OrderRequest {
//                 data: TradeOrderData::from_trade_order(order, false, "IOC".into()),
//                 signature: format!("0x{}", hex::encode(signature.as_bytes())),
//             };

//             let t0 = std::time::Instant::now();
//             let res = client
//                 .post(format!("{}/v1/order", config.rest_url))
//                 .json(&order)
//                 .send()
//                 .await
//                 .unwrap();
//             let rtt = t0.elapsed();

//             let status = res.status();
//             let body: serde_json::Value = res.json().await.unwrap();

//             println!("#{i} status: {status}, RTT: {rtt:?}");
//             println!("#{i} body: {body}");
//         }
//     }

//     #[tokio::test]
//     async fn cancel_all_pending() {
//         let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY env var required");

//         let config = crate::settings::Config::testnet(private_key);
//         let signer = crate::signer::Signer::new(&config.signer_config);
//         let domain = crate::make_domain(config.chain_id, config.exchange.parse().unwrap());
//         let client = reqwest::Client::new();

//         let ts = crate::models::Timestamp::now();
//         let (cancel_sig, order) = signer.sign_cancel_order(ts.nonce, &domain);
//         let cancel_data = CancelOrderData::from_cancel_order(
//             order,
//             vec![
//                 "57622c04-733e-4b47-af70-93a135531512".parse().unwrap(),
//                 "c4701162-6972-4aff-aa81-1a222bd4af2f".parse().unwrap(),
//             ],
//             vec![],
//         );

//         let cancel_req = CancelOrderRequest {
//             data: cancel_data,
//             signature: format!("0x{}", hex::encode(cancel_sig.as_bytes())),
//         };

//         let res = client
//             .post(format!("{}/v1/order/cancel", config.rest_url))
//             .json(&cancel_req)
//             .send()
//             .await
//             .unwrap();

//         println!("cancel: {}", res.text().await.unwrap());
//     }

//     #[tokio::test]
//     async fn test_ws_connect() {
//         use futures_util::{SinkExt, StreamExt};
//         use tokio_tungstenite::connect_async;
//         use tokio_tungstenite::tungstenite::Message;

//         let (ws_stream, _) =
//             connect_async("wss://ws.etherealtest.net/socket.io/?EIO=4&transport=websocket")
//                 .await
//                 .unwrap();
//         let (mut write, mut read) = ws_stream.split();

//         // Читаем handshake
//         if let Some(Ok(msg)) = read.next().await {
//             println!("handshake: {msg}");
//         }

//         write
//             .send(Message::Text("40/v1/stream,".into()))
//             .await
//             .unwrap();

//         if let Some(Ok(msg)) = read.next().await {
//             println!("namespace connected: {msg}");
//         }
//     }

//     #[tokio::test]
//     async fn test_ws_order_update() {
//         use futures_util::{SinkExt, StreamExt};
//         use tokio_tungstenite::connect_async;
//         use tokio_tungstenite::tungstenite::Message;

//         let (ws_stream, _) =
//             connect_async("wss://ws.etherealtest.net/socket.io/?EIO=4&transport=websocket")
//                 .await
//                 .unwrap();
//         let (mut write, mut read) = ws_stream.split();

//         // Handshake
//         read.next().await;

//         // Connect namespace
//         write
//             .send(Message::Text("40/v1/stream,".into()))
//             .await
//             .unwrap();
//         read.next().await;

//         // Subscribe ORDER_UPDATE
//         let subscribe = r#"42/v1/stream,["subscribe",{"type":"OrderUpdate","subaccountId":"48119502-2465-45c5-970e-27a28a4e0e3c"}]"#;
//         write.send(Message::Text(subscribe.into())).await.unwrap();

//         // Читаем сообщения с обработкой ping
//         for _ in 0..10 {
//             match read.next().await {
//                 Some(Ok(Message::Text(text))) => {
//                     if text == "2" {
//                         // Engine.IO ping -> pong
//                         write.send(Message::Text("3".into())).await.unwrap();
//                         println!("ping/pong");
//                     } else {
//                         println!("msg: {text}");
//                     }
//                 }
//                 Some(Ok(msg)) => println!("other: {msg:?}"),
//                 _ => break,
//             }
//         }
//     }
// }
