use confy;
use serde_derive::{Deserialize, Serialize};
use std::env;
// MQTT Topics
pub const TOPIC_DID: &str = "did";
pub const TOPIC_SENSOR_VALUE: &str = "sensors";
pub const TOPIC_SETTING: &str = "settings";
pub const TOPIC_IDENTITY: &str = "identity";
pub const TOPIC_STREAM: &str = "stream";
pub const TOPIC_COMMAND: &str = "command";
pub const PUBLIC_CHANNEL_ID: &str = "public_stream";
// Gateway
// Number of Subscribers Expected
pub const MQTT_SOCKET: &str = "0.0.0.0:50054";
/// Default Streams Service GRPC Socket
pub const STREAMS_SOCKET: &str = "0.0.0.0:50052";
/// Default Identity Service GRPC Socket
pub const IDENTITY_SOCKET: &str = "0.0.0.0:50053";
pub const ENV_IS_FACTORY: &str = "IS_FACTORY"; // bool "true", "false"
pub const ENV_ANNLINK_PUBLIC: &str = "PUBLIC_ANN_LINK";
pub const ENV_DEVICE_NAME: &str = "DEVICE_NAME";
pub const ENV_DEVICE_TYPE: &str = "DEVICE_TYPE";
pub const ENV_DEVICE_ID: &str = "DEVICE_ID";
pub const ENV_CHANNELS_KEY: &str = "CHANNEL_IDS"; // Seperated with ';'
pub const ENV_THING_KEY: &str = "THING_NAME";
pub const ENV_THING_PWD: &str = "THING_PWD";
#[allow(dead_code)]
pub const ENV_LATEST_TIMESTAMP: &str = "LATEST_TIMESTAMP";
pub const ENV_SENSOR_KEYS: &str = "SENSOR_IDS"; // Seperated with ';'
/// ENV for GRPC Socket
const ENV_GRPC_SOCKET: &str = "GATEWAY_GRPC_SOCKET";
/// Default GRPC Socket
const DEFAULT_GRPC_SOCKET: &str = "[::0]:50051";
/// Structure used to parse configuration file
#[derive(Debug, Serialize, Deserialize)]
pub struct SensorConfig {
    pub grpc: Grpc,
}
/// Socket needed for GRPC server, for example 0.0.0.0:50051
#[derive(Debug, Serialize, Deserialize)]
pub struct Grpc {
    pub socket: String,
}
/// Default implementation uses socket at 0.0.0.0:50051, default can be set via ENVs
/// and Sensor information
impl Default for SensorConfig {
    fn default() -> Self {
        SensorConfig {
            grpc: Grpc {
                socket: env::var(ENV_GRPC_SOCKET)
                    .unwrap_or_else(|_| DEFAULT_GRPC_SOCKET.to_string()),
            },
        }
    }
}
/// Configuration file "streams-grpc.toml" is located at ./config/
/// Function tries to load configuration or creates default
#[allow(dead_code)]
pub fn load_config_file() -> SensorConfig {
    let config_dir = env::current_dir()
        .unwrap()
        .join("config")
        .join("client-grpc.toml");
    let res = confy::load_path(config_dir);
    let cfg: SensorConfig = match res {
        Ok(r) => r,
        Err(_e) => SensorConfig::default(),
    };
    cfg
}
