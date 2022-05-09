#![allow(dead_code)]
use confy;
use serde_derive::{Deserialize, Serialize};
use std::env;

/// ENV for Stronghold Password
pub const ENV_STRONGHOLD_PWD: &str = "IDENTITY_STRONGHOLD_PWD";
/// Default Stronghold Password
pub const DEFAULT_STRONGHOLD_PWD: &str = "123456";
/// Folder to hold Stronghold Storage
pub const STRONGHOLD_FOLDER: &str = "stronghold";
/// File Name of Stronghold Storage
pub const STRONGHOLD_FILE: &str = "identity.hodl";

/// ENV for Sign Method Name
const ENV_IDENTITY_FRAGMENT: &str = "IDENTITY_FRAGMENT";
/// Default Sign Method Name
const DEFAULT_IDENTITY_FRAGMENT: &str = "dev-1";
/// ENV for Credential Description (Type)
const ENV_IDENTITY_CRED_TYPE: &str = "IDENTITY_CRED_TYPE";
/// Default Credential Description (Type)
const DEFAULT_IDENTITY_CRED_TYPE: &str = "Device Identification";

/// Structure used to parse configuration file
#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub identity: Identity,
}
/// Device Indentity (DID and VC), Sign Method Name and Credential Description
#[derive(Debug, Serialize, Deserialize)]
pub struct Identity {
    pub did: String,
    pub verifiable_credential: String,
    pub fragment: String,
    pub cred_type: String,
}
/// Default implementation for Configuration File, default can be set via ENVs
impl Default for IdentityConfig {
    fn default() -> Self {
        IdentityConfig {
            identity: Identity {
                did: "".to_string(),
                verifiable_credential: "".to_string(),
                fragment: env::var(ENV_IDENTITY_FRAGMENT)
                    .unwrap_or_else(|_| DEFAULT_IDENTITY_FRAGMENT.to_string()),
                cred_type: env::var(ENV_IDENTITY_CRED_TYPE)
                    .unwrap_or_else(|_| DEFAULT_IDENTITY_CRED_TYPE.to_string()),
            },
        }
    }
}
/// Saving changes made in IdentityConfig structure to configuration file "identity-grpc.toml"
/// located at ./config/
pub fn save_config_file(cfg: IdentityConfig) -> Result<String, String> {
    let config_dir = env::current_dir()
        .unwrap()
        .join("identity.toml");
    let res = confy::store_path(config_dir, cfg);
    match res {
        Ok(_) => return Ok("Config file saved".to_string()),
        Err(e) => return Err(format!("Error saving config file: {}", e)),
    };
}
/// Configuration file "identity-grpc.toml" is located at ./config/
/// Function tries to load configuration or creates default
pub fn load_config_file() -> IdentityConfig {
    let config_dir = env::current_dir()
        .unwrap()
        .join("identity.toml");
    let res = confy::load_path(config_dir);
    let cfg: IdentityConfig = match res {
        Ok(r) => r,
        Err(_e) => IdentityConfig::default(),
    };
    cfg
}
