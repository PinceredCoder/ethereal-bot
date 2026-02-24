use crate::EtherealRuntime;
use crate::settings::Config;

#[tokio::test]
async fn measure_post_only_latency_with_ws() {
    let private_key = std::env::var("PRIVATE_KEY").expect("PRIVATE_KEY required");
    let config = Config::testnet(private_key);
    let mut bot = EtherealRuntime::new(&config).await.unwrap();

    bot.subscribe_order_updates("48119502-2465-45c5-970e-27a28a4e0e3c")
        .await
        .unwrap();

    let price_raw: u128 = 60_000 * 1_000_000_000;
    let qty_raw: u128 = 1_000_000_000 / 10_000;

    println!(
        "{:<5} {:<14} {:<14} {:<14} {:<14}",
        "iter", "rest_rtt", "ws_rtt", "ws_after_rest", "engine_ms"
    );
    println!("{}", "-".repeat(65));

    for i in 0..5 {
        let (client_order_id, t0, ws_receiver) = bot
            .place_order(price_raw, qty_raw, 0, 1, true, "GTD")
            .await
            .unwrap();
        let rest_rtt = t0.elapsed();

        let (update, ws_received_at) = ws_receiver.await.unwrap();
        let ws_rtt = ws_received_at - t0;
        let ws_after_rest_ms = ws_rtt.as_millis() as i64 - rest_rtt.as_millis() as i64;
        let engine_ms = update.updated_at.abs_diff(update.created_at);

        println!(
            "{:<5} {:<14} {:<14} {:<14} {:<14}",
            format!("#{i}"),
            format!("{rest_rtt:.1?}"),
            format!("{ws_rtt:.1?}"),
            format!("{ws_after_rest_ms:+}ms"),
            format!("{engine_ms}ms"),
        );

        bot.cancel_order(client_order_id).await.unwrap();
    }
}
