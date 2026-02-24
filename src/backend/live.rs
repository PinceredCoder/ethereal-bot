use super::{BackendFuture, OrderBackend};
use crate::error::EtherealRuntimeError;
use crate::models::dto::{CancelOrderRequest, CancelOrderResult, OrderRequest, SubmitOrderResult};

pub(crate) struct LiveBackend {
    http_client: reqwest::Client,
    rest_url: url::Url,
}

impl LiveBackend {
    pub fn new(http_client: reqwest::Client, rest_url: url::Url) -> Self {
        Self {
            http_client,
            rest_url,
        }
    }

    fn map_transport_error(err: reqwest::Error) -> EtherealRuntimeError {
        if err.is_builder() || err.is_request() || err.is_connect() {
            EtherealRuntimeError::RequestNotSent(err)
        } else {
            EtherealRuntimeError::RequestDeliveryUncertain(err)
        }
    }
}

impl OrderBackend for LiveBackend {
    fn submit_order<'a>(
        &'a self,
        request: &'a OrderRequest,
    ) -> BackendFuture<'a, SubmitOrderResult> {
        Box::pin(async move {
            let res = self
                .http_client
                .post(format!("{}/v1/order", self.rest_url))
                .json(request)
                .send()
                .await
                .map_err(Self::map_transport_error)?;

            let payload: serde_json::Value = res
                .json()
                .await
                .map_err(EtherealRuntimeError::RequestDeliveryUncertain)?;

            if payload.get("result").and_then(|r| r.as_str()) == Some("Ok") {
                Ok(SubmitOrderResult::Accepted { payload })
            } else {
                Ok(SubmitOrderResult::Rejected { payload })
            }
        })
    }

    fn cancel_order<'a>(
        &'a self,
        request: &'a CancelOrderRequest,
    ) -> BackendFuture<'a, CancelOrderResult> {
        Box::pin(async move {
            let res = self
                .http_client
                .post(format!("{}/v1/order/cancel", self.rest_url))
                .json(request)
                .send()
                .await
                .map_err(Self::map_transport_error)?;

            let payload: serde_json::Value = res
                .json()
                .await
                .map_err(EtherealRuntimeError::RequestDeliveryUncertain)?;

            if payload.get("result").and_then(|r| r.as_str()) == Some("Ok") {
                Ok(CancelOrderResult::Accepted { payload })
            } else {
                Ok(CancelOrderResult::Rejected { payload })
            }
        })
    }
}
