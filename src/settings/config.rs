use url::Url;

use crate::signer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExecutionMode {
    Live,
    #[default]
    Paper,
}

#[derive(serde::Deserialize)]
pub struct Config {
    pub rest_url: Url,
    pub ws_url: Url,
    pub chain_id: u64,
    pub exchange: alloy::primitives::Address,
    #[serde(default)]
    pub execution_mode: ExecutionMode,

    pub signer_config: signer::Config,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn new() -> Config {
        let env_config = config::Environment::default()
            .separator("__")
            .list_separator(";")
            .with_list_parse_key("auth_settings.allowed_origins")
            .with_list_parse_key("rpc_settings.states_rpc_endpoints")
            .try_parsing(true);

        let mut conf_builder = config::Config::builder().add_source(env_config);

        if std::path::Path::new("Settings.toml").exists() {
            conf_builder = conf_builder.add_source(config::File::with_name("./Settings.toml"));
        }

        conf_builder
            .build()
            .unwrap()
            .try_deserialize::<Config>()
            .unwrap_or_else(|e| panic!("Error parsing config: {e}"))
    }

    pub fn testnet(private_key: String) -> Self {
        Self {
            rest_url: "https://api.etherealtest.net".parse().unwrap(),
            ws_url: "wss://ws.etherealtest.net".parse().unwrap(),
            chain_id: 13374202,
            exchange: "1F0327A80e43FEF1Cd872DC5d38dCe4A165c0643".parse().unwrap(),
            execution_mode: ExecutionMode::Live,
            signer_config: signer::Config {
                subaccount: hex::decode(
                    "7072696d61727900000000000000000000000000000000000000000000000000",
                )
                .unwrap(),
                private_key: hex::decode(private_key).unwrap(),
            },
        }
    }
}
