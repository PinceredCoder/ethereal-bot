mod error;
mod executor;
mod logging;
mod models;
mod runtime;
mod settings;
mod signer;

#[cfg(test)]
mod tests;

pub use error::EtherealRuntimeError;
pub use logging::{LoggingConfig, LoggingGuards, init_logging};
pub use runtime::EtherealRuntime;
pub use settings::{Config, ExecutionMode};

pub async fn build_runtime(config: &Config) -> Result<EtherealRuntime, EtherealRuntimeError> {
    EtherealRuntime::new(config).await
}
