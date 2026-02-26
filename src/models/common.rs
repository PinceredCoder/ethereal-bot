#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Default,
    serde::Deserialize,
    serde::Serialize,
    strum::AsRefStr,
    strum::Display,
)]
#[serde(rename_all = "UPPERCASE")]
#[strum(serialize_all = "UPPERCASE")]
pub enum TimeInForce {
    #[default]
    Gtd,
    Ioc,
    Fok,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    strum::AsRefStr,
    strum::Display,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    New,
    Pending,
    FilledPartial,
    Filled,
    Rejected,
    Canceled,
    Expired,
}
