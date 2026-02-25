use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

#[derive(Debug, Deserialize)]
struct ProductListEnvelope {
    data: Vec<ProductEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProductEntry {
    id: String,
}

#[tokio::test]
#[ignore = "manual websocket sampling test; run explicitly when capturing WS payloads"]
async fn sample_market_price_ws_payloads() {
    let rest_url = std::env::var("ETHEREAL_REST_URL")
        .unwrap_or_else(|_| "https://api.ethereal.trade".to_string());
    let ws_url =
        std::env::var("ETHEREAL_WS_URL").unwrap_or_else(|_| "wss://ws.ethereal.trade".to_string());

    let product_id = match std::env::var("ETHEREAL_TEST_PRODUCT_ID") {
        Ok(id) => id,
        Err(_) => first_product_id(&rest_url).await,
    };

    println!("sampling MarketPrice for productId={product_id}");

    let socket_url = Url::parse(&ws_url)
        .unwrap()
        .join("socket.io/?EIO=4&transport=websocket")
        .unwrap();

    let (ws_stream, response) = tokio_tungstenite::connect_async(socket_url.to_string())
        .await
        .unwrap();
    println!("ws connected with status={}", response.status());

    let (mut write, mut read) = ws_stream.split();

    if let Some(Ok(Message::Text(open))) =
        timeout(Duration::from_secs(5), read.next()).await.unwrap()
    {
        println!("handshake frame: {open}");
    }

    write
        .send(Message::Text("40/v1/stream,".into()))
        .await
        .unwrap();

    if let Some(Ok(Message::Text(ack))) =
        timeout(Duration::from_secs(5), read.next()).await.unwrap()
    {
        println!("namespace ack: {ack}");
    }

    let subscribe = format!(
        r#"42/v1/stream,["subscribe",{{"type":"MarketPrice","productId":"{product_id}"}}]"#
    );
    write.send(Message::Text(subscribe.into())).await.unwrap();
    println!("sent subscribe for MarketPrice");

    let mut captured_frames = Vec::new();
    let mut saw_market_price_frame = false;

    while captured_frames.len() < 6 {
        let next = timeout(Duration::from_secs(20), read.next())
            .await
            .expect("timed out waiting for websocket frame");
        let Some(frame) = next else {
            break;
        };
        let frame = frame.expect("websocket receive error");

        match frame {
            Message::Text(text) => {
                if text == "2" {
                    write.send(Message::Text("3".into())).await.unwrap();
                    continue;
                }

                if text.starts_with("42/v1/stream,") {
                    if let Some(payload) = text.strip_prefix("42/v1/stream,") {
                        if let Ok(parsed) = serde_json::from_str::<Value>(payload) {
                            println!(
                                "event frame parsed: {}",
                                serde_json::to_string_pretty(&parsed).unwrap()
                            );
                            if parsed.to_string().contains("MarketPrice") {
                                saw_market_price_frame = true;
                            }
                        } else {
                            println!("event frame raw: {text}");
                        }
                    }
                } else {
                    println!("text frame raw: {text}");
                }

                captured_frames.push(text.to_string());
            }
            Message::Binary(bytes) => {
                println!("binary frame ({} bytes)", bytes.len());
            }
            Message::Ping(payload) => {
                write.send(Message::Pong(payload)).await.unwrap();
            }
            Message::Pong(_) => {}
            Message::Frame(_) => {}
            Message::Close(reason) => {
                println!("close frame: {reason:?}");
                break;
            }
        }
    }

    println!("captured {} non-heartbeat frames", captured_frames.len());
    assert!(
        !captured_frames.is_empty(),
        "no non-heartbeat frames were received; cannot infer schema"
    );
    assert!(
        saw_market_price_frame,
        "captured frames did not contain MarketPrice marker; inspect printed payloads"
    );
}

async fn first_product_id(rest_url: &str) -> String {
    let url = format!("{rest_url}/v1/product?limit=1");
    let response = reqwest::Client::new().get(url).send().await.unwrap();
    let response = response.error_for_status().unwrap();
    let payload: ProductListEnvelope = response.json().await.unwrap();

    payload
        .data
        .first()
        .expect("no products returned by /v1/product")
        .id
        .clone()
}
