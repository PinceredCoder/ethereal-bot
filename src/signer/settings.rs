#[derive(serde::Deserialize)]
pub struct Config {
    #[serde(with = "hex::serde")]
    pub private_key: Vec<u8>,
    #[serde(with = "hex::serde")]
    pub subaccount: Vec<u8>,
}
