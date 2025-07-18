use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub device: String,
    pub rom: String,
    pub max_retries: u32,
    pub version: String,
    pub android_version: u32,
    pub timestamp: String,
    pub variant: String,
    pub patches: Vec<String>,
    pub signing: Option<SigningConfig>,
    pub output: OutputConfig,
    pub expected_checksum: Option<String>,
    #[serde(default = "default_cleanup")]
    pub cleanup: bool,
    #[serde(default)]
    pub hooks: Hooks,
}

fn default_cleanup() -> bool {
    true
}

#[derive(Debug, Deserialize, Default)]
pub struct Hooks {
    #[serde(flatten)]
    pub scripts: HashMap<String, String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct SigningConfig {
    pub method: String,
    pub keystore_path: String,
    pub key_alias: String,
    pub keystore_password: String,
    pub key_password: String,
    pub custom_command: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct OutputConfig {
    pub filename: String,
}
