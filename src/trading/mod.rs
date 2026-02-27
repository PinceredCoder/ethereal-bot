pub(crate) mod policy;
pub(crate) mod settings;
pub(crate) mod state;

use tokio::sync::mpsc;

use self::policy::StrategyAction;
use self::settings::StrategyConfig;
use self::state::{Side, SideState, StrategyState};
use crate::logging::targets;
use crate::models::common::OrderStatus;
use crate::models::dto::{MarketPriceData, OrderUpdateData};
use crate::{EtherealRuntime, EtherealRuntimeError, RuntimeEvent};

pub async fn run_strategy_loop(
    runtime: &EtherealRuntime,
    config: &StrategyConfig,
    mut market_events: mpsc::UnboundedReceiver<RuntimeEvent>,
) -> Result<(), EtherealRuntimeError> {
    runtime.subscribe_order_updates(config.subaccount).await?;
    runtime.subscribe_market_price(config.product_id).await?;
    let mut state = StrategyState::default();

    while let Some(event) = market_events.recv().await {
        let mut latest_market_tick = None;
        match event {
            RuntimeEvent::MarketPrice(tick) => {
                if tick.product_id == config.product_id {
                    latest_market_tick = Some(tick);
                }
            }
            RuntimeEvent::OrderUpdate(update) => {
                reconcile_order_update(&mut state, &update);
            }
        }

        while let Ok(next_event) = market_events.try_recv() {
            match next_event {
                RuntimeEvent::MarketPrice(tick) => {
                    if tick.product_id == config.product_id {
                        latest_market_tick = Some(tick);
                    }
                }
                RuntimeEvent::OrderUpdate(update) => {
                    reconcile_order_update(&mut state, &update);
                }
            }
        }

        if let Some(tick) = latest_market_tick
            && let Err(error) = handle_market_tick(runtime, config, &mut state, &tick).await
        {
            tracing::warn!(
                target: targets::TRADING_DECISION,
                %error,
                "strategy tick processing failed"
            );
        }
    }

    Err(EtherealRuntimeError::WS(
        "runtime event stream closed".to_string(),
    ))
}

fn reconcile_order_update(state: &mut StrategyState, update: &OrderUpdateData) {
    let buy_matched = reconcile_side_order_update(state.side_state_mut(Side::Buy), update);
    let sell_matched = reconcile_side_order_update(state.side_state_mut(Side::Sell), update);
    if buy_matched || sell_matched {
        tracing::info!(
            target: targets::TRADING_DECISION,
            client_order_id = %update.client_order_id,
            status = %update.status,
            "strategy state reconciled from order update"
        );
    }
}

fn reconcile_side_order_update(side_state: &mut SideState, update: &OrderUpdateData) -> bool {
    if side_state.active_client_order_id != Some(update.client_order_id) {
        return false;
    }

    side_state.inflight = false;
    if is_terminal_order_status(update.status) {
        side_state.active_client_order_id = None;
        side_state.last_quoted_price_raw = None;
    }

    true
}

fn is_terminal_order_status(status: OrderStatus) -> bool {
    match status {
        OrderStatus::Filled
        | OrderStatus::Rejected
        | OrderStatus::Canceled
        | OrderStatus::Expired => true,
        OrderStatus::New | OrderStatus::Pending | OrderStatus::FilledPartial => false,
    }
}

pub async fn handle_market_tick(
    runtime: &EtherealRuntime,
    config: &StrategyConfig,
    state: &mut StrategyState,
    tick: &MarketPriceData,
) -> Result<(), EtherealRuntimeError> {
    if tick.product_id != config.product_id {
        return Ok(());
    }

    state.last_market = Some(tick.clone());
    let (buy_action, sell_action) = policy::decide_actions(config, state, tick);

    execute_action(runtime, config, state, Side::Buy, buy_action).await?;
    execute_action(runtime, config, state, Side::Sell, sell_action).await?;

    Ok(())
}

async fn execute_action(
    runtime: &EtherealRuntime,
    config: &StrategyConfig,
    state: &mut StrategyState,
    side: Side,
    action: Option<StrategyAction>,
) -> Result<(), EtherealRuntimeError> {
    let Some(action) = action else {
        tracing::debug!(
            target: targets::TRADING_DECISION,
            %side,
            "strategy action: skip"
        );
        return Ok(());
    };

    match action {
        StrategyAction::Place { price_raw, qty_raw } => {
            place_side_order(runtime, config, state, side, price_raw, qty_raw).await
        }
        StrategyAction::Cancel { client_order_id } => {
            cancel_side_order(runtime, state, side, client_order_id).await
        }
        StrategyAction::Replace {
            old_client_order_id,
            new_price_raw,
            qty_raw,
        } => {
            cancel_side_order(runtime, state, side, old_client_order_id).await?;
            place_side_order(runtime, config, state, side, new_price_raw, qty_raw).await
        }
    }
}

async fn place_side_order(
    runtime: &EtherealRuntime,
    config: &StrategyConfig,
    state: &mut StrategyState,
    side: Side,
    price_raw: u128,
    qty_raw: u128,
) -> Result<(), EtherealRuntimeError> {
    state.side_state_mut(side).inflight = true;

    let client_order_id = runtime
        .place_order(
            price_raw,
            qty_raw,
            side as u8,
            config.onchain_product_id,
            config.post_only,
            config.time_in_force,
        )
        .await?;

    state.side_state_mut(side).inflight = false;

    let side_state = state.side_state_mut(side);
    side_state.active_client_order_id = Some(client_order_id);
    side_state.last_quoted_price_raw = Some(price_raw);

    tracing::info!(
        target: targets::TRADING_DECISION,
        %side,
        %client_order_id,
        price_raw,
        qty_raw,
        "strategy action: place (optimistic completion)"
    );

    Ok(())
}

async fn cancel_side_order(
    runtime: &EtherealRuntime,
    state: &mut StrategyState,
    side: Side,
    client_order_id: uuid::Uuid,
) -> Result<(), EtherealRuntimeError> {
    state.side_state_mut(side).inflight = true;
    let cancel_result = runtime.cancel_order(client_order_id).await;
    state.side_state_mut(side).inflight = false;
    cancel_result?;

    let side_state = state.side_state_mut(side);
    side_state.active_client_order_id = None;
    side_state.last_quoted_price_raw = None;

    tracing::info!(
        target: targets::TRADING_DECISION,
        %side,
        %client_order_id,
        "strategy action: cancel (optimistic completion)"
    );

    Ok(())
}
