use super::{ExecutorError, OrderExecutor, is_submit_accepted};
use crate::logging::targets;
use crate::models::dto::{CancelOrderRequest, OrderRequest};

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
    ) -> Result<serde_json::Value, ExecutorError> {
        let payload = serde_json::json!({ "data": &request.data });
        let response = self
            .http_client
            .post(format!("{}/v1/order/dry-run", self.rest_url))
            .json(&payload)
            .send()
            .await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await?;
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/dry-run",
                status = %status,
                payload = %body,
                "paper submit rejected"
            );
            return Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: body,
            });
        }

        let payload: serde_json::Value = response.json().await?;

        if is_submit_accepted(&payload) {
            tracing::info!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/dry-run",
                status = %status,
                "paper submit accepted"
            );
            Ok(payload)
        } else {
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/dry-run",
                status = %status,
                payload = %payload,
                "paper submit rejected"
            );
            Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: payload.to_string(),
            })
        }
    }

    async fn cancel_order(
        &self,
        _request: &CancelOrderRequest,
    ) -> Result<serde_json::Value, ExecutorError> {
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

        Err(ExecutorError::Rejected {
            status: 422,
            payload: payload.to_string(),
        })
    }
}
