use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub device: String,
    pub rom: String,
    pub version: String,
    pub android_version: String,
    pub patches: Vec<String>,
    pub signing: Option<SigningConfig>,
    pub output: OutputConfig,
}

#[derive(Debug, Deserialize)]
pub struct SigningConfig {
    pub enabled: bool,
    pub key_path: Option<String>,
    pub cert_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub filename: String,
    pub compression_level: Option<u8>,
}

