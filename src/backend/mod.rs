use std::future::Future;
use std::pin::Pin;

use crate::error::EtherealBotError;
use crate::models::{CancelOrderRequest, OrderRequest};
use crate::settings::ExecutionMode;

// TODO(step-3): remove this allowance once OrderBackend is wired into EtherealBot.
#[allow(dead_code)]
pub(crate) type BackendFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, EtherealBotError>> + Send + 'a>>;

// TODO(step-3): remove this allowance once concrete backends are implemented and used.
#[allow(dead_code)]
pub(crate) trait OrderBackend: Send + Sync {
    fn mode(&self) -> ExecutionMode;

    fn submit_order<'a>(
        &'a self,
        request: &'a OrderRequest,
    ) -> BackendFuture<'a, serde_json::Value>;

    fn cancel_order<'a>(&'a self, request: &'a CancelOrderRequest) -> BackendFuture<'a, ()>;
}
