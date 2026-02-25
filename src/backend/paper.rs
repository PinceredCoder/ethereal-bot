use super::{OrderBackend, is_cancel_accepted, is_submit_accepted, map_transport_error};
use crate::error::EtherealRuntimeError;
use crate::models::dto::{CancelOrderRequest, CancelOrderResult, OrderRequest, SubmitOrderResult};

pub(crate) struct PaperBackend {
    http_client: reqwest::Client,
    rest_url: url::Url,
}

impl PaperBackend {
    pub fn new(http_client: reqwest::Client, rest_url: url::Url) -> Self {
        Self {
            http_client,
            rest_url,
        }
    }
}

impl OrderBackend for PaperBackend {
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
            .map_err(map_transport_error)?
            .error_for_status()
            .map_err(map_transport_error)?;

        let payload: serde_json::Value = res
            .json()
            .await
            .map_err(EtherealRuntimeError::RequestDeliveryUncertain)?;

        if is_submit_accepted(&payload) {
            Ok(SubmitOrderResult::Accepted { payload })
        } else {
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
                "message": "cancel is not supported for paper backend yet"
            }]
        });

        if is_cancel_accepted(&payload) {
            Ok(CancelOrderResult::Accepted { payload })
        } else {
            Ok(CancelOrderResult::Rejected { payload })
        }
    }
}
