use serde::Serialize;
use uuid::Uuid;

use super::contracts::{CancelOrder, TradeOrder};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeOrderData {
    pub sender: String,
    pub subaccount: String,
    pub quantity: String,
    pub price: String,
    pub reduce_only: bool,
    pub side: u8,
    pub engine_type: u8,
    pub onchain_id: u32,
    pub nonce: String,
    pub signed_at: u64,
    #[serde(rename = "type")]
    pub order_type: String,
    pub time_in_force: &'static str,
    pub post_only: bool,
    pub client_order_id: Uuid,
}

impl TradeOrderData {
    pub fn from_trade_order(
        order: TradeOrder,
        post_only: bool,
        time_in_force: &'static str,
    ) -> Self {
        Self {
            sender: format!("{:?}", order.sender),
            subaccount: format!("0x{}", hex::encode(order.subaccount)),
            quantity: (order.quantity as f64 / 1_000_000_000.0).to_string(),
            price: (order.price / 1_000_000_000).to_string(),
            reduce_only: order.reduceOnly,
            side: order.side,
            engine_type: 0,
            onchain_id: order.productId,
            nonce: order.nonce.to_string(),
            signed_at: order.signedAt,
            order_type: "LIMIT".to_string(),
            time_in_force,
            post_only,
            client_order_id: Uuid::new_v4(),
        }
    }
}

pub struct Timestamp {
    pub nonce: u64,
    pub signed_at: u64,
}

impl Timestamp {
    pub fn now() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        Self {
            nonce: now.as_nanos() as u64,
            signed_at: now.as_secs(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OrderRequest {
    pub data: TradeOrderData,
    pub signature: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelOrderData {
    pub sender: String,
    pub subaccount: String,
    pub nonce: String,
    pub order_ids: Vec<Uuid>,
    pub client_order_ids: Vec<Uuid>,
}

impl CancelOrderData {
    pub fn from_cancel_order(
        order: CancelOrder,
        order_ids: Vec<Uuid>,
        client_order_ids: Vec<Uuid>,
    ) -> Self {
        Self {
            sender: format!("{:?}", order.sender),
            subaccount: format!("0x{}", hex::encode(order.subaccount)),
            nonce: order.nonce.to_string(),
            order_ids,
            client_order_ids,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CancelOrderRequest {
    pub data: CancelOrderData,
    pub signature: String,
}

// TODO(step-3): remove this allowance once backend implementations construct these results.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum SubmitOrderResult {
    Accepted { payload: serde_json::Value },
    Rejected { payload: serde_json::Value },
}

// TODO(step-3): remove this allowance once backend implementations construct these results.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum CancelOrderResult {
    Accepted { payload: serde_json::Value },
    Rejected { payload: serde_json::Value },
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderUpdateData {
    pub id: Uuid,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub client_order_id: Uuid,
}

#[derive(Debug, serde::Deserialize)]
pub struct WsEnvelope {
    pub data: Vec<OrderUpdateData>,
}

pub fn parse_order_update(msg: &str) -> Option<WsEnvelope> {
    let payload = msg.strip_prefix("42/v1/stream,")?;
    let (_, envelope): (&str, WsEnvelope) = serde_json::from_str(payload).ok()?;
    Some(envelope)
}
