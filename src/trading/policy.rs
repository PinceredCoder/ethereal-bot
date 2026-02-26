use num_traits::ToPrimitive;

use super::settings::StrategyConfig;
use super::state::{Side, SideState, StrategyState};
use crate::models::dto::MarketPriceData;

pub type SideActions = (Option<StrategyAction>, Option<StrategyAction>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyAction {
    Place {
        price_raw: u128,
        qty_raw: u128,
    },
    Cancel {
        client_order_id: uuid::Uuid,
    },
    Replace {
        old_client_order_id: uuid::Uuid,
        new_price_raw: u128,
        qty_raw: u128,
    },
}

pub fn decide_actions(
    config: &StrategyConfig,
    state: &StrategyState,
    tick: &MarketPriceData,
) -> SideActions {
    if tick.product_id != config.product_id {
        return (None, None);
    }

    let Some(best_bid_raw) = decimal_to_raw(&tick.best_bid_price) else {
        return (None, None);
    };
    let Some(best_ask_raw) = decimal_to_raw(&tick.best_ask_price) else {
        return (None, None);
    };

    let spread_raw = best_ask_raw.saturating_sub(best_bid_raw);
    let min_spread_raw = config
        .tick_size_raw
        .saturating_mul(config.min_spread_ticks as u128);

    if best_ask_raw <= best_bid_raw || (config.min_spread_ticks > 0 && spread_raw < min_spread_raw)
    {
        return (
            cancel_if_active(state.side_state(Side::Buy)),
            cancel_if_active(state.side_state(Side::Sell)),
        );
    }

    let desired_buy_raw = quantize_to_tick(best_bid_raw, config.tick_size_raw);
    let desired_sell_raw = quantize_to_tick(best_ask_raw, config.tick_size_raw);

    (
        decide_side_action(state.side_state(Side::Buy), desired_buy_raw, config.qty_raw),
        decide_side_action(
            state.side_state(Side::Sell),
            desired_sell_raw,
            config.qty_raw,
        ),
    )
}

fn decide_side_action(
    side_state: &SideState,
    desired_price_raw: u128,
    qty_raw: u128,
) -> Option<StrategyAction> {
    if side_state.inflight {
        return None;
    }

    match side_state.active_client_order_id {
        None => Some(StrategyAction::Place {
            price_raw: desired_price_raw,
            qty_raw,
        }),
        Some(active_id) => {
            if side_state.last_quoted_price_raw == Some(desired_price_raw) {
                None
            } else {
                Some(StrategyAction::Replace {
                    old_client_order_id: active_id,
                    new_price_raw: desired_price_raw,
                    qty_raw,
                })
            }
        }
    }
}

fn cancel_if_active(side_state: &SideState) -> Option<StrategyAction> {
    if side_state.inflight {
        return None;
    }

    side_state
        .active_client_order_id
        .map(|client_order_id| StrategyAction::Cancel { client_order_id })
}

fn quantize_to_tick(price_raw: u128, tick_size_raw: u128) -> u128 {
    if tick_size_raw == 0 {
        return price_raw;
    }
    (price_raw / tick_size_raw) * tick_size_raw
}

fn decimal_to_raw(value: &bigdecimal::BigDecimal) -> Option<u128> {
    let scaled = value.with_scale(crate::models::util::ORDER_DECIMAL_PLACES as i64);
    let (digits, _scale) = scaled.into_bigint_and_scale();
    digits.to_biguint()?.to_u128()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use bigdecimal::BigDecimal;
    use uuid::Uuid;

    use super::{StrategyAction, decide_actions, decimal_to_raw};
    use crate::models::dto::MarketPriceData;
    use crate::trading::settings::{StrategyConfig, TimeInForce};
    use crate::trading::state::StrategyState;

    #[test]
    fn converts_decimal_to_raw_using_scale() {
        let value = BigDecimal::from_str("123.4567890129").unwrap();
        assert_eq!(decimal_to_raw(&value), Some(123_456_789_012));
    }

    #[test]
    fn emits_place_actions_for_empty_state() {
        let product_id = Uuid::new_v4();
        let config = base_config(product_id);
        let state = StrategyState::default();
        let tick = market(product_id, "100", "101");

        let actions = decide_actions(&config, &state, &tick);
        assert_eq!(
            actions,
            (
                Some(StrategyAction::Place {
                    price_raw: 100_000_000_000,
                    qty_raw: config.qty_raw,
                }),
                Some(StrategyAction::Place {
                    price_raw: 101_000_000_000,
                    qty_raw: config.qty_raw,
                })
            )
        );
    }

    #[test]
    fn emits_none_for_unchanged_prices() {
        let product_id = Uuid::new_v4();
        let config = base_config(product_id);
        let mut state = StrategyState::default();
        state.buy.active_client_order_id = Some(Uuid::new_v4());
        state.sell.active_client_order_id = Some(Uuid::new_v4());
        state.buy.last_quoted_price_raw = Some(100_000_000_000);
        state.sell.last_quoted_price_raw = Some(101_000_000_000);
        let tick = market(product_id, "100", "101");

        assert_eq!(decide_actions(&config, &state, &tick), (None, None));
    }

    #[test]
    fn emits_replace_when_price_changes() {
        let product_id = Uuid::new_v4();
        let config = base_config(product_id);
        let mut state = StrategyState::default();
        let buy_order_id = Uuid::new_v4();
        state.buy.active_client_order_id = Some(buy_order_id);
        state.buy.last_quoted_price_raw = Some(99_000_000_000);
        let tick = market(product_id, "100", "101");

        let actions = decide_actions(&config, &state, &tick);
        assert_eq!(
            actions,
            (
                Some(StrategyAction::Replace {
                    old_client_order_id: buy_order_id,
                    new_price_raw: 100_000_000_000,
                    qty_raw: config.qty_raw,
                }),
                Some(StrategyAction::Place {
                    price_raw: 101_000_000_000,
                    qty_raw: config.qty_raw,
                })
            )
        );
    }

    #[test]
    fn spread_guard_cancels_existing_active_order() {
        let product_id = Uuid::new_v4();
        let mut config = base_config(product_id);
        config.min_spread_ticks = 2;
        let mut state = StrategyState::default();
        let buy_order_id = Uuid::new_v4();
        state.buy.active_client_order_id = Some(buy_order_id);
        let tick = market(product_id, "100", "101");

        assert_eq!(
            decide_actions(&config, &state, &tick),
            (
                Some(StrategyAction::Cancel {
                    client_order_id: buy_order_id,
                }),
                None
            )
        );
    }

    #[test]
    fn inflight_side_is_blocked() {
        let product_id = Uuid::new_v4();
        let config = base_config(product_id);
        let mut state = StrategyState::default();
        state.buy.inflight = true;
        let tick = market(product_id, "100", "101");

        assert_eq!(
            decide_actions(&config, &state, &tick),
            (
                None,
                Some(StrategyAction::Place {
                    price_raw: 101_000_000_000,
                    qty_raw: config.qty_raw,
                })
            )
        );
    }

    fn base_config(product_id: Uuid) -> StrategyConfig {
        StrategyConfig {
            subaccount: Uuid::new_v4(),
            product_id,
            onchain_product_id: 42,
            qty_raw: 100_000_000,
            post_only: true,
            time_in_force: TimeInForce::Gtd,
            tick_size_raw: 1_000_000_000,
            min_spread_ticks: 1,
        }
    }

    fn market(product_id: Uuid, bid: &str, ask: &str) -> MarketPriceData {
        MarketPriceData {
            product_id,
            best_bid_price: BigDecimal::from_str(bid).unwrap(),
            best_ask_price: BigDecimal::from_str(ask).unwrap(),
            oracle_price: BigDecimal::from_str("100").unwrap(),
            price24h_ago: BigDecimal::from_str("99").unwrap(),
        }
    }
}
