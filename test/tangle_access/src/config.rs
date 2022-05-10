use confy;
use serde_derive::{Deserialize, Serialize};
use std::env;

pub const ENV_TEST_IDENTITIES: &str = "TEST_IDENTITIES";

/// Structure used to parse configuration file
#[derive(Debug, Serialize, Deserialize)]
pub struct IdentityConfig {
    pub identity: Identity,
}
/// Device Indentity (DID and VC), Sign Method Name and Credential Description
#[derive(Debug, Serialize, Deserialize)]
pub struct Identity {
    pub did: String,
    pub vc: String,
}
/// Default implementation for Configuration File, default can be set via ENVs
impl Default for IdentityConfig {
    fn default() -> Self {
        IdentityConfig {
            identity: Identity {
                did: "".to_string(),
                vc: "".to_string(),
            },
        }
    }
}
/// Saving changes made in IdentityConfig structure to configuration file "identity-grpc.toml"
pub fn save_config_file(cfg: IdentityConfig, filename: &str) -> Result<String, String> {
    let config_dir = env::current_dir()
        .unwrap()
        .join(format!("{}.toml", filename));
    let res = confy::store_path(config_dir, cfg);
    match res {
        Ok(_) => return Ok("Config file saved".to_string()),
        Err(e) => return Err(format!("Error saving config file: {}", e)),
    };
}
/// Configuration file
/// Function tries to load configuration or creates default
pub fn load_config_file(filename: &str) -> IdentityConfig {
    let config_dir = env::current_dir()
        .unwrap()
        .join(format!("{}.toml", filename));
    let res = confy::load_path(config_dir);
    let cfg: IdentityConfig = match res {
        Ok(r) => r,
        Err(_e) => IdentityConfig::default(),
    };
    cfg
}
