#[derive(Debug, thiserror::Error)]
pub enum EtherealBotError {
    #[error("invalid url: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error(transparent)]
    WS(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("failed connecting to the exchange: {0}")]
    Connection(String),

    #[error("invalid address: {0}")]
    InvalidHexAddress(#[from] alloy::hex::FromHexError),

    #[error(transparent)]
    HttpError(#[from] reqwest::Error),

    #[error("exchange API error: {0}")]
    Api(String),
}
