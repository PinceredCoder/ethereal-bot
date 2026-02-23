use uuid::Uuid;

use crate::EtherealBot;
use crate::settings::Config;

#[derive(Debug, serde::Deserialize)]
struct WsEnvelope {
    pub data: Vec<OrderUpdateData>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderUpdateData {
    pub id: Uuid,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
}

fn parse_order_update(msg: &str) -> Option<WsEnvelope> {
    // Срезаем "42/v1/stream," и парсим ["OrderUpdate", {...}] как tuple
    let payload = msg.strip_prefix("42/v1/stream,")?;
    let (_, envelope): (&str, WsEnvelope) = serde_json::from_str(payload).ok()?;
    Some(envelope)
}

#[tokio::test]
async fn test_ws_connect() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    let (ws_stream, _) =
        connect_async("wss://ws.etherealtest.net/socket.io/?EIO=4&transport=websocket")
            .await
            .unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Читаем handshake
    if let Some(Ok(msg)) = read.next().await {
        println!("handshake: {msg}");
    }

    write
        .send(Message::Text("40/v1/stream,".into()))
        .await
        .unwrap();

    if let Some(Ok(msg)) = read.next().await {
        println!("namespace connected: {msg}");
    }
}

#[tokio::test]
async fn measure_post_only_latency_with_ws() {
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY required");
    let config = Config::testnet(private_key);
    let mut bot = EtherealBot::new(&config).await.unwrap();

    bot.subscribe_order_updates("48119502-2465-45c5-970e-27a28a4e0e3c")
        .await
        .unwrap();

    let price_raw: u128 = 60_000 * 1_000_000_000;
    let qty_raw: u128 = 1_000_000_000 / 10_000;

    println!(
        "{:<5} {:<14} {:<14} {:<14}",
        "iter", "rest_rtt", "engine_ms", "ws_rtt"
    );
    println!("{}", "-".repeat(50));

    for i in 0..5 {
        let (order_id, t0) = bot
            .place_order(price_raw, qty_raw, 0, 1, true, "GTD")
            .await
            .unwrap();
        let rest_rtt = t0.elapsed();

        loop {
            let msg = bot.next_ws_message().await.unwrap().unwrap();
            if !msg.contains(&order_id.to_string()) {
                continue;
            }
            if let Some(payload) = parse_order_update(&msg) {
                for update in &payload.data {
                    if update.id == order_id && update.status == "NEW" {
                        let ws_rtt = t0.elapsed();
                        let engine_ms = update.updated_at.saturating_sub(update.created_at);
                        println!(
                            "{:<5} {:<14} {:<14} {:<14}",
                            format!("#{i}"),
                            format!("{rest_rtt:.1?}"),
                            format!("{engine_ms}ms"),
                            format!("{ws_rtt:.1?}"),
                        );
                        break;
                    }
                }
                break;
            }
        }

        bot.cancel_order(order_id).await.unwrap();
    }
}
