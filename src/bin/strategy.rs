#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ethereal_bot::Config::new();
    let _logging_guards = ethereal_bot::init_logging(&config.logging);

    ethereal_bot::run_strategy(&config).await?;

    Ok(())
}
