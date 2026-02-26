mod error;
mod live;
mod paper;

pub use error::ExecutorError;

use crate::models::dto::{CancelOrderRequest, OrderRequest};

pub(crate) trait OrderExecutor: Send + Sync {
    async fn submit_order(
        &self,
        request: &OrderRequest,
    ) -> Result<serde_json::Value, ExecutorError>;

    async fn cancel_order(
        &self,
        request: &CancelOrderRequest,
    ) -> Result<serde_json::Value, ExecutorError>;
}

pub(crate) fn is_cancel_accepted(payload: &serde_json::Value) -> bool {
    let Some(items) = payload.get("data").and_then(|value| value.as_array()) else {
        return false;
    };

    if items.is_empty() {
        return false;
    }

    items.iter().all(|item| {
        matches!(
            item.get("result").and_then(|value| value.as_str()),
            Some("Ok")
                | Some("AlreadyCanceled")
                | Some("AlreadyExpired")
                | Some("AlreadyFilled")
                | Some("NotFound")
        )
    })
}

pub(crate) enum OrderExecutorRuntime {
    Live(LiveExecutor),
    Paper(PaperExecutor),
}

impl OrderExecutorRuntime {
    pub(crate) async fn submit_order(
        &self,
        request: &OrderRequest,
    ) -> Result<serde_json::Value, ExecutorError> {
        match self {
            Self::Live(executor) => executor.submit_order(request).await,
            Self::Paper(executor) => executor.submit_order(request).await,
        }
    }

    pub(crate) async fn cancel_order(
        &self,
        request: &CancelOrderRequest,
    ) -> Result<serde_json::Value, ExecutorError> {
        match self {
            Self::Live(executor) => executor.cancel_order(request).await,
            Self::Paper(executor) => executor.cancel_order(request).await,
        }
    }
}

pub(crate) use live::LiveExecutor;
pub(crate) use paper::PaperExecutor;

#[cfg(test)]
mod tests {
    use super::is_cancel_accepted;

    #[test]
    fn cancel_accepts_ok_and_idempotent_results() {
        let payload = serde_json::json!({
            "data": [
                { "id": "9036443a-441a-4a66-87f2-bd5c44cdca7a", "result": "Ok" },
                { "id": "9036443a-441a-4a66-87f2-bd5c44cdca7a", "result": "AlreadyCanceled" },
                { "id": "9036443a-441a-4a66-87f2-bd5c44cdca7a", "result": "NotFound" }
            ]
        });

        assert!(is_cancel_accepted(&payload));
    }

    #[test]
    fn cancel_rejects_unknown_or_nonce_already_used() {
        let unknown = serde_json::json!({
            "data": [{ "id": "9036443a-441a-4a66-87f2-bd5c44cdca7a", "result": "Unknown" }]
        });
        let nonce = serde_json::json!({
            "data": [{ "id": "9036443a-441a-4a66-87f2-bd5c44cdca7a", "result": "NonceAlreadyUsed" }]
        });

        assert!(!is_cancel_accepted(&unknown));
        assert!(!is_cancel_accepted(&nonce));
    }
}
