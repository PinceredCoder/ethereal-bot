#[derive(Debug, thiserror::Error)]
pub enum EtherealRuntimeError {
    #[error("invalid url: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("websocket error: {0}")]
    WS(String),

    #[error("invalid address: {0}")]
    InvalidHexAddress(#[from] alloy::hex::FromHexError),

    #[error("execution mode `{0}` is not implemented yet")]
    ExecutionModeNotImplemented(&'static str),

    #[error(transparent)]
    Executor(#[from] crate::ExecutorError),
}

impl From<tokio_tungstenite::tungstenite::Error> for EtherealRuntimeError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WS(err.to_string())
    }
}
