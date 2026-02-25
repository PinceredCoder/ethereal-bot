use bigdecimal::BigDecimal;
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

#[derive(Debug, Clone, PartialEq)]
pub enum SubmitOrderResult {
    Accepted { payload: serde_json::Value },
    Rejected { payload: serde_json::Value },
}

#[derive(Debug, Clone, PartialEq)]
pub enum CancelOrderResult {
    Accepted { payload: serde_json::Value },
    Rejected { payload: serde_json::Value },
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderUpdateData {
    pub id: Uuid,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub client_order_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketPriceData {
    pub product_id: Uuid,
    pub best_bid_price: BigDecimal,
    pub best_ask_price: BigDecimal,
    pub oracle_price: BigDecimal,
    #[serde(rename = "price24hAgo")]
    pub price24h_ago: BigDecimal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WsEvent {
    OrderUpdate(Vec<OrderUpdateData>),
    MarketPrice(Vec<MarketPriceData>),
    Unknown {
        event: String,
        payload: serde_json::Value,
    },
}

pub fn parse_ws_event(msg: &str) -> Option<WsEvent> {
    let payload = msg.strip_prefix("42/v1/stream,")?;
    let (event, payload): (String, serde_json::Value) = serde_json::from_str(payload).ok()?;

    match event.as_str() {
        "OrderUpdate" => parse_event_items(payload).map(WsEvent::OrderUpdate),
        "MarketPrice" => parse_event_items(payload).map(WsEvent::MarketPrice),
        _ => Some(WsEvent::Unknown { event, payload }),
    }
}

fn parse_event_items<T: serde::de::DeserializeOwned>(payload: serde_json::Value) -> Option<Vec<T>> {
    match payload {
        serde_json::Value::Array(_) => serde_json::from_value(payload).ok(),
        serde_json::Value::Object(mut object) => {
            if let Some(data_payload) = object.remove("data") {
                match data_payload {
                    serde_json::Value::Array(_) => serde_json::from_value(data_payload).ok(),
                    _ => serde_json::from_value(data_payload)
                        .ok()
                        .map(|item| vec![item]),
                }
            } else {
                serde_json::from_value(serde_json::Value::Object(object))
                    .ok()
                    .map(|item| vec![item])
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use uuid::Uuid;

    use super::{WsEvent, parse_ws_event};

    #[test]
    fn parse_order_update_object_payload() {
        let msg = concat!(
            "42/v1/stream,",
            r#"["OrderUpdate",{"data":[{"id":"11111111-1111-1111-1111-111111111111","status":"OPEN","createdAt":1712019600000,"updatedAt":1712019600100,"clientOrderId":"22222222-2222-2222-2222-222222222222"}]}]"#
        );

        let event = parse_ws_event(msg).expect("expected parsed event");
        match event {
            WsEvent::OrderUpdate(updates) => {
                assert_eq!(updates.len(), 1);
                let update = &updates[0];
                assert_eq!(update.status, "OPEN");
                assert_eq!(
                    update.client_order_id,
                    Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap()
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parse_market_price_array_payload() {
        let msg = concat!(
            "42/v1/stream,",
            r#"["MarketPrice",[{"bestAskPrice":"65107","bestBidPrice":"65102","oraclePrice":"65087.60585299","price24hAgo":"63202.61954649","productId":"bc7d5575-3711-4532-a000-312bfacfb767"}]]"#
        );

        let event = parse_ws_event(msg).expect("expected parsed event");
        match event {
            WsEvent::MarketPrice(items) => {
                assert_eq!(items.len(), 1);
                let item = &items[0];
                assert_eq!(
                    item.product_id,
                    Uuid::parse_str("bc7d5575-3711-4532-a000-312bfacfb767").unwrap()
                );
                assert_eq!(item.best_bid_price, BigDecimal::from_str("65102").unwrap());
                assert_eq!(item.best_ask_price, BigDecimal::from_str("65107").unwrap());
                assert_eq!(
                    item.oracle_price,
                    BigDecimal::from_str("65087.60585299").unwrap()
                );
                assert_eq!(
                    item.price24h_ago,
                    BigDecimal::from_str("63202.61954649").unwrap()
                );
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parse_market_price_single_object_payload() {
        let msg = concat!(
            "42/v1/stream,",
            r#"["MarketPrice",{"bestAskPrice":"65107","bestBidPrice":"65102","oraclePrice":"65087.60585299","price24hAgo":"63202.61954649","productId":"bc7d5575-3711-4532-a000-312bfacfb767"}]"#
        );

        let event = parse_ws_event(msg).expect("expected parsed event");
        match event {
            WsEvent::MarketPrice(items) => assert_eq!(items.len(), 1),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parse_unknown_event() {
        let msg = r#"42/v1/stream,["FooEvent",{"k":"v"}]"#;
        let event = parse_ws_event(msg).expect("expected parsed event");

        match event {
            WsEvent::Unknown { event, payload } => {
                assert_eq!(event, "FooEvent");
                assert_eq!(payload["k"], "v");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn parse_non_namespace_returns_none() {
        let msg = r#"42/other,["MarketPrice",[]]"#;
        assert!(parse_ws_event(msg).is_none());
    }

    #[test]
    fn parse_malformed_json_returns_none() {
        let msg = r#"42/v1/stream,["MarketPrice",[{"productId":"bad"}"#;
        assert!(parse_ws_event(msg).is_none());
    }
}
