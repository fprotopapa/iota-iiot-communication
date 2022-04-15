#![allow(dead_code)]
use confy;
use serde_derive::{Deserialize, Serialize};
use std::env;

/// ENV for GRPC Socket
const ENV_GRPC_SOCKET: &str = "IDENTITY_GRPC_SOCKET";
/// Default GRPC Socket
const DEFAULT_GRPC_SOCKET: &str = "[::1]:50054";
/// MQTT Subscriber Message Stream Buffer Size
pub const MESSAGE_BUFFER_SIZE: usize = 25;
/// ENV Name for MQTT Server Address
pub const ENV_SERVER_URL: &str = "MQTT_SERVER_URL";
/// ENV Name for MQTT Server Port
pub const ENV_SERVER_PORT: &str = "MQTT_SERVER_PORT";
/// Default MQTT Server Port
pub const DEFAULT_SERVER_PORT: &str = "8883";
/// Default MQTT Server Address
pub const DEFAULT_SERVER_URL: &str = "broker.emqx.io";
/// ENV Name for Channel ID
pub const ENV_CHANNEL_ID: &str = "CHANNEL_ID";
/// ENV Name for Thing ID
pub const ENV_THING_ID: &str = "THING_NAME";
/// ENV Name for Thing Password
pub const ENV_THING_PWD: &str = "THING_PWD";
/// ENV Name for Client Thing ID
pub const ENV_CLIENT_THING_ID: &str = "THING_CLI_NAME";
/// ENV Name for Client Thing Password
pub const ENV_CLIENT_THING_PWD: &str = "THING_CLI_PWD";

/// Structure used to parse configuration file
#[derive(Debug, Serialize, Deserialize)]
pub struct MqttConfig {
    pub grpc: Grpc,
    pub mqtt: Mqtt,
}
/// MQTT Settings
#[derive(Debug, Serialize, Deserialize)]
pub struct Mqtt {
    pub tls: bool,
    pub port: u32,
    pub ca_name: String,
    pub mqtt_v5: bool,
    pub recon_intv: (u64, u64),
    pub keep_alive: u64,
    pub timeout: u64,
    pub clean_session: bool,
    pub qos_pub: i32,
    pub qos: Vec<i32>,
    pub topics: Vec<String>,
}
/// Socket needed for GRPC server, for example \[::1]:50051
#[derive(Debug, Serialize, Deserialize)]
pub struct Grpc {
    pub socket: String,
}
/// Default implementation for Configuration File, default can be set via ENVs
impl Default for MqttConfig {
    fn default() -> Self {
        MqttConfig {
            grpc: Grpc {
                socket: env::var(ENV_GRPC_SOCKET)
                    .unwrap_or_else(|_| DEFAULT_GRPC_SOCKET.to_string()),
            },
            mqtt: Mqtt {
                tls: true,
                port: env::var(ENV_SERVER_PORT)
                    .unwrap_or_else(|_| DEFAULT_SERVER_PORT.to_string())
                    .trim()
                    .parse()
                    .expect("Unable to Parse Port Number"),
                ca_name: "ca.crt".to_string(),
                mqtt_v5: false,
                recon_intv: (2, 20),
                keep_alive: 20,
                timeout: 5,
                clean_session: false,
                qos_pub: 1,
                qos: vec![1; 5],
                topics: vec![
                    "did".to_string(),
                    "identity".to_string(),
                    "stream".to_string(),
                    "sensors".to_string(),
                    "settings".to_string(),
                    "command".to_string(),
                ],
            },
        }
    }
}
/// Saving changes made in IdentityConfig structure to configuration file "mqtt-grpc.toml"
/// located at ./config/
pub fn save_config_file(cfg: MqttConfig) -> Result<String, String> {
    let config_dir = env::current_dir()
        .unwrap()
        .join("config")
        .join("mqtt-grpc.toml");
    let res = confy::store_path(config_dir, cfg);
    match res {
        Ok(_) => return Ok("Config file saved".to_string()),
        Err(e) => return Err(format!("Error saving config file: {}", e)),
    };
}
/// Configuration file "mqtt-grpc.toml" is located at ./config/
/// Function tries to load configuration or creates default
pub fn load_config_file() -> MqttConfig {
    let config_dir = env::current_dir()
        .unwrap()
        .join("config")
        .join("mqtt-grpc.toml");
    let res = confy::load_path(config_dir);
    let cfg: MqttConfig = match res {
        Ok(r) => r,
        Err(_e) => MqttConfig::default(),
    };
    cfg
}
