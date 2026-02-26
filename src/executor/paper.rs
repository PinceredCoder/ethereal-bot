use super::{OrderExecutor, is_cancel_accepted, is_submit_accepted, map_transport_error};
use crate::error::EtherealRuntimeError;
use crate::logging::targets;
use crate::models::dto::{CancelOrderRequest, CancelOrderResult, OrderRequest, SubmitOrderResult};

pub(crate) struct PaperExecutor {
    http_client: reqwest::Client,
    rest_url: url::Url,
}

impl PaperExecutor {
    pub fn new(http_client: reqwest::Client, rest_url: url::Url) -> Self {
        Self {
            http_client,
            rest_url,
        }
    }
}

impl OrderExecutor for PaperExecutor {
    async fn submit_order(
        &self,
        request: &OrderRequest,
    ) -> Result<SubmitOrderResult, EtherealRuntimeError> {
        let payload = serde_json::json!({ "data": &request.data });
        let res = self
            .http_client
            .post(format!("{}/v1/order/dry-run", self.rest_url))
            .json(&payload)
            .send()
            .await
            .map_err(map_transport_error)?;
        let status = res.status();
        let res = res.error_for_status().map_err(map_transport_error)?;

        let payload: serde_json::Value = res
            .json()
            .await
            .map_err(EtherealRuntimeError::RequestDeliveryUncertain)?;

        if is_submit_accepted(&payload) {
            tracing::info!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/dry-run",
                status = %status,
                "paper submit accepted"
            );
            Ok(SubmitOrderResult::Accepted { payload })
        } else {
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/dry-run",
                status = %status,
                payload = %payload,
                "paper submit rejected"
            );
            Ok(SubmitOrderResult::Rejected { payload })
        }
    }

    async fn cancel_order(
        &self,
        _request: &CancelOrderRequest,
    ) -> Result<CancelOrderResult, EtherealRuntimeError> {
        let payload = serde_json::json!({
            "data": [{
                "result": "PaperCancelUnsupported",
                "message": "cancel is not supported for paper executor yet"
            }]
        });

        tracing::warn!(
            target: targets::RUNTIME_EXEC,
            "paper cancel requested but unsupported"
        );

        if is_cancel_accepted(&payload) {
            Ok(CancelOrderResult::Accepted { payload })
        } else {
            Ok(CancelOrderResult::Rejected { payload })
        }
    }
}
