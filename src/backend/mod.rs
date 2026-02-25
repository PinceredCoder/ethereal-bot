mod live;
mod paper;

use crate::error::EtherealRuntimeError;
use crate::models::dto::{CancelOrderRequest, CancelOrderResult, OrderRequest, SubmitOrderResult};

pub(crate) trait OrderBackend: Send + Sync {
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

pub(crate) enum OrderBackendRuntime {
    Live(LiveBackend),
    Paper(PaperBackend),
}

impl OrderBackendRuntime {
    pub(crate) async fn submit_order(
        &self,
        request: &OrderRequest,
    ) -> Result<SubmitOrderResult, EtherealRuntimeError> {
        match self {
            Self::Live(backend) => backend.submit_order(request).await,
            Self::Paper(backend) => backend.submit_order(request).await,
        }
    }

    pub(crate) async fn cancel_order(
        &self,
        request: &CancelOrderRequest,
    ) -> Result<CancelOrderResult, EtherealRuntimeError> {
        match self {
            Self::Live(backend) => backend.cancel_order(request).await,
            Self::Paper(backend) => backend.cancel_order(request).await,
        }
    }
}

pub(crate) use live::LiveBackend;
pub(crate) use paper::PaperBackend;
