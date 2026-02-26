mod error;
mod executor;
mod logging;
mod models;
mod runtime;
mod settings;
mod signer;
mod trading;

#[cfg(test)]
mod tests;

use error::EtherealRuntimeError;
use executor::ExecutorError;
pub use logging::{LoggingGuards, init_logging};
use runtime::{EtherealRuntime, RuntimeEvent};
pub use settings::Config;

async fn build_runtime(
    config: &Config,
) -> Result<
    (
        EtherealRuntime,
        tokio::sync::mpsc::UnboundedReceiver<RuntimeEvent>,
    ),
    EtherealRuntimeError,
> {
    EtherealRuntime::new(config).await
}

pub async fn run_strategy(config: &Config) -> Result<(), EtherealRuntimeError> {
    let (runtime, market_events) = build_runtime(config).await?;
    trading::run_strategy_loop(&runtime, &config.strategy, market_events).await
}
