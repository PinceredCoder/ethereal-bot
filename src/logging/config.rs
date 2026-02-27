use serde::Deserialize;
use tracing_appender::rolling::Rotation;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_directory")]
    pub directory: String,
    #[serde(default = "default_technical_file")]
    pub technical_file: String,
    #[serde(default = "default_decision_file")]
    pub decision_file: String,
    #[serde(
        default = "default_log_rotation",
        deserialize_with = "deserialize_rotation"
    )]
    pub rotation: Rotation,
    #[serde(default = "default_file_level", deserialize_with = "deserialize_level")]
    pub file_level: tracing::Level,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            directory: default_directory(),
            technical_file: default_technical_file(),
            decision_file: default_decision_file(),
            rotation: default_log_rotation(),
            file_level: default_file_level(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

fn default_directory() -> String {
    "logs".to_string()
}

fn default_technical_file() -> String {
    "runtime.log".to_string()
}

fn default_decision_file() -> String {
    "decisions.log".to_string()
}

fn default_file_level() -> tracing::Level {
    tracing::Level::INFO
}

fn default_log_rotation() -> Rotation {
    Rotation::DAILY
}

fn deserialize_level<'de, D>(deserializer: D) -> Result<tracing::Level, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let normalized = raw.trim().to_ascii_uppercase();
    normalized.parse().map_err(serde::de::Error::custom)
}

fn deserialize_rotation<'de, D>(deserializer: D) -> Result<Rotation, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let normalized = raw.trim().to_ascii_uppercase();

    match normalized.as_str() {
        "MINUTELY" => Ok(Rotation::MINUTELY),
        "HOURLY" => Ok(Rotation::HOURLY),
        "DAILY" => Ok(Rotation::DAILY),
        "WEEKLY" => Ok(Rotation::WEEKLY),
        "NEVER" => Ok(Rotation::NEVER),
        _ => Err(serde::de::Error::custom("unknown log rotation type")),
    }
}
