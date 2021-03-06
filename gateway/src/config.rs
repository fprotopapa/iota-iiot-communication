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
// Gateway
// Number of Subscribers Expected
pub const MQTT_SOCKET: &str = "0.0.0.0:50054";
/// Default Streams Service GRPC Socket
pub const STREAMS_SOCKET: &str = "0.0.0.0:50052";
/// Default Identity Service GRPC Socket
pub const IDENTITY_SOCKET: &str = "0.0.0.0:50053";
pub const ENV_TOTAL_NUM_SUBSCRIBER: &str = "NUM_SUBSCRIBER";
pub const ENV_DEVICE_NAME: &str = "DEVICE_NAME";
pub const ENV_DEVICE_TYPE: &str = "DEVICE_TYPE";
pub const ENV_DEVICE_ID: &str = "DEVICE_ID";
pub const ENV_CHANNEL_KEY: &str = "CHANNEL_ID";
pub const ENV_THING_KEY: &str = "THING_NAME";
pub const ENV_THING_PWD: &str = "THING_PWD";
/// ENV for GRPC Socket
const ENV_GRPC_SOCKET: &str = "GATEWAY_GRPC_SOCKET";
/// Default GRPC Socket
const DEFAULT_GRPC_SOCKET: &str = "0.0.0.0:50051";
/// Default for Channel Buffer Size
pub const DEFAULT_BUFFER_SIZE: usize = 32;
/// Structure used to parse configuration file
#[derive(Debug, Serialize, Deserialize)]
pub struct SensorConfig {
    pub grpc: Grpc,
    pub sensors: Sensors,
}
/// Socket needed for GRPC server, for example \[::1]:50051
#[derive(Debug, Serialize, Deserialize)]
pub struct Grpc {
    pub socket: String,
}
/// Used Sensors
#[derive(Debug, Serialize, Deserialize)]
pub struct Sensors {
    pub list: Vec<Sensor>,
}
/// Sensor Information
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Sensor {
    pub sensor_id: String,
    pub sensor_name: String,
    pub type_descr: String,
    pub unit: String,
}
/// Default implementation uses socket at \[::1]:50051, default can be set via ENVs
/// and Sensor information
impl Default for SensorConfig {
    fn default() -> Self {
        SensorConfig {
            grpc: Grpc {
                socket: env::var(ENV_GRPC_SOCKET)
                    .unwrap_or_else(|_| DEFAULT_GRPC_SOCKET.to_string()),
            },
            sensors: Sensors {
                list: vec![
                    Sensor {
                        sensor_id: "12".to_string(),
                        sensor_name: "MPC 7812".to_string(),
                        type_descr: "Temperature".to_string(),
                        unit: "C".to_string(),
                    },
                    Sensor {
                        sensor_id: "5".to_string(),
                        sensor_name: "DHT 15".to_string(),
                        type_descr: "Humidity".to_string(),
                        unit: "%".to_string(),
                    },
                ],
            },
        }
    }
}
/// Configuration file "streams-grpc.toml" is located at ./config/
/// Function tries to load configuration or creates default
pub fn load_config_file() -> SensorConfig {
    let config_dir = env::current_dir()
        .unwrap()
        .join("config")
        .join("gateway-grpc.toml");
    let res = confy::load_path(config_dir);
    let cfg: SensorConfig = match res {
        Ok(r) => r,
        Err(_e) => SensorConfig::default(),
    };
    cfg
}
