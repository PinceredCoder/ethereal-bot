mod live;
mod paper;

use crate::error::EtherealRuntimeError;
use crate::models::dto::{CancelOrderRequest, CancelOrderResult, OrderRequest, SubmitOrderResult};

pub(crate) trait OrderExecutor: Send + Sync {
    async fn submit_order(
        &self,
        request: &OrderRequest,
    ) -> Result<SubmitOrderResult, EtherealRuntimeError>;

    async fn cancel_order(
        &self,
        request: &CancelOrderRequest,
    ) -> Result<CancelOrderResult, EtherealRuntimeError>;
}

pub(crate) fn map_transport_error(err: reqwest::Error) -> EtherealRuntimeError {
    if err.is_builder() || err.is_request() || err.is_connect() {
        EtherealRuntimeError::RequestNotSent(err)
    } else {
        EtherealRuntimeError::RequestDeliveryUncertain(err)
    }
}

pub(crate) fn is_submit_accepted(payload: &serde_json::Value) -> bool {
    payload
        .get("result")
        .and_then(|value| value.as_str())
        .or_else(|| payload.get("code").and_then(|value| value.as_str()))
        == Some("Ok")
}

pub(crate) fn is_cancel_accepted(payload: &serde_json::Value) -> bool {
    let Some(items) = payload.get("data").and_then(|value| value.as_array()) else {
        return false;
    };

    if items.is_empty() {
        return false;
    }

    items
        .iter()
        .all(|item| item.get("result").and_then(|value| value.as_str()) == Some("Ok"))
}

pub(crate) enum OrderExecutorRuntime {
    Live(LiveExecutor),
    Paper(PaperExecutor),
}

impl OrderExecutorRuntime {
    pub(crate) async fn submit_order(
        &self,
        request: &OrderRequest,
    ) -> Result<SubmitOrderResult, EtherealRuntimeError> {
        match self {
            Self::Live(executor) => executor.submit_order(request).await,
            Self::Paper(executor) => executor.submit_order(request).await,
        }
    }

    pub(crate) async fn cancel_order(
        &self,
        request: &CancelOrderRequest,
    ) -> Result<CancelOrderResult, EtherealRuntimeError> {
        match self {
            Self::Live(executor) => executor.cancel_order(request).await,
            Self::Paper(executor) => executor.cancel_order(request).await,
        }
    }
}

pub(crate) use live::LiveExecutor;
pub(crate) use paper::PaperExecutor;
