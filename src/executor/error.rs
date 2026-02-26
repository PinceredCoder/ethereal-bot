#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("request send/receive failed: {0}")]
    SendRequestError(String),
    #[error(transparent)]
    HttpError(#[from] reqwest::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error("request rejected by exchange (status: {status:?})")]
    Rejected { status: u16, payload: String },
}
