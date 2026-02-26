use super::{OrderExecutor, is_cancel_accepted, is_submit_accepted, map_transport_error};
use crate::error::EtherealRuntimeError;
use crate::logging::targets;
use crate::models::dto::{CancelOrderRequest, CancelOrderResult, OrderRequest, SubmitOrderResult};

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
    ) -> Result<SubmitOrderResult, EtherealRuntimeError> {
        let res = self
            .http_client
            .post(format!("{}/v1/order", self.rest_url))
            .json(request)
            .send()
            .await
            .map_err(map_transport_error)?;
        let status = res.status();

        let payload: serde_json::Value = res
            .json()
            .await
            .map_err(EtherealRuntimeError::RequestDeliveryUncertain)?;

        if is_submit_accepted(&payload) {
            tracing::info!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order",
                status = %status,
                "live submit accepted"
            );
            Ok(SubmitOrderResult::Accepted { payload })
        } else {
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order",
                status = %status,
                payload = %payload,
                "live submit rejected"
            );
            Ok(SubmitOrderResult::Rejected { payload })
        }
    }

    async fn cancel_order(
        &self,
        request: &CancelOrderRequest,
    ) -> Result<CancelOrderResult, EtherealRuntimeError> {
        let res = self
            .http_client
            .post(format!("{}/v1/order/cancel", self.rest_url))
            .json(request)
            .send()
            .await
            .map_err(map_transport_error)?;
        let status = res.status();

        let payload: serde_json::Value = res
            .json()
            .await
            .map_err(EtherealRuntimeError::RequestDeliveryUncertain)?;

        if is_cancel_accepted(&payload) {
            tracing::info!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/cancel",
                status = %status,
                "live cancel accepted"
            );
            Ok(CancelOrderResult::Accepted { payload })
        } else {
            tracing::warn!(
                target: targets::RUNTIME_EXEC,
                endpoint = "/v1/order/cancel",
                status = %status,
                payload = %payload,
                "live cancel rejected"
            );
            Ok(CancelOrderResult::Rejected { payload })
        }
    }
}
