use super::{ExecutorError, OrderExecutor, is_cancel_accepted, is_submit_accepted};
use crate::logging::targets;
use crate::models::dto::{CancelOrderRequest, OrderRequest};

pub(crate) struct LiveExecutor {
    http_client: reqwest::Client,
    rest_url: url::Url,
}

impl LiveExecutor {
    pub fn new(http_client: reqwest::Client, rest_url: url::Url) -> Self {
        Self {
            http_client,
            rest_url,
        }
    }
}

impl OrderExecutor for LiveExecutor {
    async fn submit_order(
        &self,
        request: &OrderRequest,
    ) -> Result<serde_json::Value, ExecutorError> {
        let response = self
            .http_client
            .post(format!("{}/v1/order", self.rest_url))
            .json(request)
            .send()
            .await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await?;
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order",
                status = %status,
                payload = %body,
                "live submit rejected"
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
                endpoint = "/v1/order",
                status = %status,
                "live submit accepted"
            );
            Ok(payload)
        } else {
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order",
                status = %status,
                payload = %payload,
                "live submit rejected"
            );
            Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: payload.to_string(),
            })
        }
    }

    async fn cancel_order(
        &self,
        request: &CancelOrderRequest,
    ) -> Result<serde_json::Value, ExecutorError> {
        let response = self
            .http_client
            .post(format!("{}/v1/order/cancel", self.rest_url))
            .json(request)
            .send()
            .await?;
        let status = response.status();

        if !status.is_success() {
            let body = response.text().await?;
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/cancel",
                status = %status,
                payload = %body,
                "live cancel rejected"
            );
            return Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: body,
            });
        }

        let payload: serde_json::Value = response.json().await?;

        if is_cancel_accepted(&payload) {
            tracing::info!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/cancel",
                status = %status,
                "live cancel accepted"
            );
            Ok(payload)
        } else {
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/cancel",
                status = %status,
                payload = %payload,
                "live cancel rejected"
            );
            Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: payload.to_string(),
            })
        }
    }
}
