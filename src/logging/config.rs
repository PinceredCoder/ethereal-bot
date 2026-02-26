use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogRotation {
    Hourly,
    #[default]
    Daily,
    Never,
}

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
    #[serde(default)]
    pub rotation: LogRotation,
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
            rotation: LogRotation::default(),
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

fn deserialize_level<'de, D>(deserializer: D) -> Result<tracing::Level, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = String::deserialize(deserializer)?;
    let normalized = raw.trim().to_ascii_uppercase();
    normalized.parse().map_err(serde::de::Error::custom)
}
