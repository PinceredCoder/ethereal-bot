use serde::Serialize;
use uuid::Uuid;

alloy::sol! {
    #[derive(Debug)]
    struct TradeOrder {
        address sender;
        bytes32 subaccount;
        uint128 quantity;
        uint128 price;
        bool reduceOnly;
        uint8 side;
        uint8 engineType;
        uint32 productId;
        uint64 nonce;
        uint64 signedAt;
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TradeOrderData {
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
        }
    }
}

pub(crate) struct Timestamp {
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
pub(crate) struct OrderRequest {
    pub data: TradeOrderData,
    pub signature: String,
}

alloy::sol! {
    #[derive(Debug)]
    struct CancelOrder {
        address sender;
        bytes32 subaccount;
        uint64 nonce;
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelOrderData {
    pub sender: String,
    pub subaccount: String,
    pub nonce: String,
    pub order_ids: Vec<Uuid>,
    pub client_order_ids: Vec<String>,
}

impl CancelOrderData {
    pub fn from_cancel_order(
        order: CancelOrder,
        order_ids: Vec<Uuid>,
        client_order_ids: Vec<String>,
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
pub(crate) struct CancelOrderRequest {
    pub data: CancelOrderData,
    pub signature: String,
}
