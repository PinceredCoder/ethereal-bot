mod live;

use std::future::Future;
use std::pin::Pin;

use crate::error::EtherealRuntimeError;
use crate::models::dto::{CancelOrderRequest, CancelOrderResult, OrderRequest, SubmitOrderResult};

pub(crate) type BackendFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, EtherealRuntimeError>> + Send + 'a>>;

pub(crate) trait OrderBackend: Send + Sync {
    fn submit_order<'a>(
        &'a self,
        request: &'a OrderRequest,
    ) -> BackendFuture<'a, SubmitOrderResult>;

    fn cancel_order<'a>(
        &'a self,
        request: &'a CancelOrderRequest,
    ) -> BackendFuture<'a, CancelOrderResult>;
}

pub(crate) use live::LiveBackend;
