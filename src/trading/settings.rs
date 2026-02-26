use uuid::Uuid;

pub use crate::models::common::TimeInForce;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct StrategyConfig {
    pub subaccount: Uuid,
    pub product_id: Uuid,
    pub onchain_product_id: u32,
    #[serde(deserialize_with = "deserialize_u128_config")]
    pub qty_raw: u128,
    #[serde(default = "default_post_only")]
    pub post_only: bool,
    #[serde(default)]
    pub time_in_force: TimeInForce,
    #[serde(
        default = "default_tick_size_raw",
        deserialize_with = "deserialize_u128_config"
    )]
    pub tick_size_raw: u128,
    #[serde(default = "default_min_spread_ticks")]
    pub min_spread_ticks: u32,
}

fn deserialize_u128_config<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::fmt;

    use serde::de::{Error, Unexpected, Visitor};

    struct U128Visitor;

    impl<'de> Visitor<'de> for U128Visitor {
        type Value = u128;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a non-negative integer that fits in u128")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(value as u128)
        }

        fn visit_u128<E>(self, value: u128) -> Result<Self::Value, E>
        where
            E: Error,
        {
            Ok(value)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: Error,
        {
            u128::try_from(value)
                .map_err(|_| E::invalid_value(Unexpected::Signed(value), &"non-negative integer"))
        }

        fn visit_i128<E>(self, value: i128) -> Result<Self::Value, E>
        where
            E: Error,
        {
            u128::try_from(value)
                .map_err(|_| E::invalid_value(Unexpected::Other("negative i128"), &self))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: Error,
        {
            value
                .parse::<u128>()
                .map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: Error,
        {
            self.visit_str(&value)
        }
    }

    deserializer.deserialize_any(U128Visitor)
}

fn default_post_only() -> bool {
    true
}

fn default_tick_size_raw() -> u128 {
    1
}

fn default_min_spread_ticks() -> u32 {
    1
}
