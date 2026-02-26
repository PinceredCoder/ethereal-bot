use tokio::time::{Duration, Instant};

use crate::models::common::TimeInForce;
use crate::settings::Config;
use crate::{EtherealRuntime, RuntimeEvent};

#[tokio::test]
async fn measure_post_only_latency_with_ws() {
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY required");
    let config = Config::testnet(private_key);
    let (bot, mut events) = EtherealRuntime::new(&config).await.unwrap();

    bot.subscribe_order_updates("48119502-2465-45c5-970e-27a28a4e0e3c".parse().unwrap())
        .await
        .unwrap();

    let price_raw: u128 = 60_000 * 1_000_000_000;
    let qty_raw: u128 = 1_000_000_000 / 10_000;

    println!("{:<5} {:<14} {:<14}", "iter", "rest_rtt", "engine_ms");
    println!("{}", "-".repeat(40));

    for i in 0..5 {
        let t0 = Instant::now();
        let client_order_id = bot
            .place_order(price_raw, qty_raw, 0, 1, true, TimeInForce::Gtd)
            .await
            .unwrap();
        let rest_rtt = t0.elapsed();

        let update = tokio::time::timeout(Duration::from_secs(10), async {
            while let Some(event) = events.recv().await {
                if let RuntimeEvent::OrderUpdate(update) = event {
                    if update.client_order_id == client_order_id {
                        return update;
                    }
                }
            }

            panic!("runtime event stream closed while waiting for order update");
        })
        .await
        .unwrap();
        let engine_ms = update.updated_at.abs_diff(update.created_at);

        println!(
            "{:<5} {:<14} {:<14}",
            format!("#{i}"),
            format!("{rest_rtt:.1?}"),
            format!("{engine_ms}ms"),
        );

        bot.cancel_order(client_order_id).await.unwrap();
    }
}
