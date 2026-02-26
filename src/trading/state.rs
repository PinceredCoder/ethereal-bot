use uuid::Uuid;

use crate::models::dto::MarketPriceData;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::AsRefStr, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

#[derive(Debug, Clone, Default)]
pub struct SideState {
    pub active_client_order_id: Option<Uuid>,
    pub last_quoted_price_raw: Option<u128>,
    pub inflight: bool,
}

#[derive(Debug, Clone, Default)]
pub struct StrategyState {
    pub buy: SideState,
    pub sell: SideState,
    pub last_market: Option<MarketPriceData>,
}

impl StrategyState {
    pub fn side_state(&self, side: Side) -> &SideState {
        match side {
            Side::Buy => &self.buy,
            Side::Sell => &self.sell,
        }
    }

    pub fn side_state_mut(&mut self, side: Side) -> &mut SideState {
        match side {
            Side::Buy => &mut self.buy,
            Side::Sell => &mut self.sell,
        }
    }
}
