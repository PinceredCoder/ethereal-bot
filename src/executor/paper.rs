use super::{ExecutorError, OrderExecutor};
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

fn is_submit_created(payload: &serde_json::Value) -> bool {
    payload.get("marginRequired").is_some()
        && payload.get("marginAvailable").is_some()
        && payload.get("totalUsedMargin").is_some()
        && payload.get("riskUsed").is_some()
        && payload.get("riskAvailable").is_some()
        && payload.get("code").is_some()
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
            return Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: body,
            });
        }

        let payload: serde_json::Value = response.json().await?;

        if is_submit_created(&payload) {
            Ok(payload)
        } else {
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

        Err(ExecutorError::Rejected {
            status: 422,
            payload: payload.to_string(),
        })
    }
}
