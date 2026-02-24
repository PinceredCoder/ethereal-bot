#[derive(Debug, thiserror::Error)]
pub enum EtherealRuntimeError {
    #[error("invalid url: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error(transparent)]
    WS(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("failed connecting to the exchange: {0}")]
    Connection(String),

    #[error("invalid address: {0}")]
    InvalidHexAddress(#[from] alloy::hex::FromHexError),

    #[error("request was not sent to exchange: {0}")]
    RequestNotSent(#[source] reqwest::Error),

    #[error("request delivery is uncertain and may have reached exchange: {0}")]
    RequestDeliveryUncertain(#[source] reqwest::Error),

    #[error(transparent)]
    HttpError(#[from] reqwest::Error),

    #[error("execution mode `{0}` is not implemented yet")]
    ExecutionModeNotImplemented(&'static str),

    #[error("order was rejected by exchange: {0}")]
    OrderRejected(String),

    #[error("cancel request was rejected by exchange: {0}")]
    CancelRejected(String),
}
