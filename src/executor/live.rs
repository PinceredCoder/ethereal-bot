use super::{ExecutorError, OrderExecutor, is_cancel_accepted};
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

fn is_submit_created(payload: &serde_json::Value) -> bool {
    payload.get("id").is_some()
        && payload.get("filled").is_some()
        && payload.get("result").is_some()
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
            return Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: body,
            });
        }

        let payload: serde_json::Value = response.json().await?;

        if is_cancel_accepted(&payload) {
            Ok(payload)
        } else {
            Err(ExecutorError::Rejected {
                status: status.as_u16(),
                payload: payload.to_string(),
            })
        }
    }
}
