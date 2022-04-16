use confy;
use serde_derive::{Deserialize, Serialize};
use std::env;
/// Default TTY Path Unix
#[cfg(unix)]
const DEFAULT_TTY: &str = "/dev/ttyBleOut";
/// Default TTY Path Windows
#[cfg(windows)]
const DEFAULT_TTY: &str = "COM1";
/// ENV for GRPC Socket
const ENV_GRPC_SOCKET: &str = "SENSOR_GRPC_SOCKET";
/// Default GRPC Socket
const DEFAULT_GRPC_SOCKET: &str = "[::1]:50051";
/// Default for Channel Buffer Size
#[allow(dead_code)]
pub const DEFAULT_BUFFER_SIZE: usize = 32;
/// Structure used to parse configuration file
#[derive(Debug, Serialize, Deserialize)]
pub struct SensorConfig {
    pub grpc: Grpc,
    pub ble: Ble,
}
/// Socket needed for GRPC server, for example \[::1]:50051
#[derive(Debug, Serialize, Deserialize)]
pub struct Grpc {
    pub socket: String,
}
/// Sets delay and socket path
#[derive(Debug, Serialize, Deserialize)]
pub struct Ble {
    pub tty: String,
    pub delay_ms: u64,
}
/// Default implementation uses socket at \[::1]:50051, default can be set via ENVs
impl Default for SensorConfig {
    fn default() -> Self {
        SensorConfig {
            grpc: Grpc {
                socket: env::var(ENV_GRPC_SOCKET)
                    .unwrap_or_else(|_| DEFAULT_GRPC_SOCKET.to_string()),
            },
            ble: Ble {
                tty: DEFAULT_TTY.to_string(),
                delay_ms: 5000,
            },
        }
    }
}
/// Configuration file "sensor-bluetooth-grpc.toml" is located at ./config/
/// Function tries to load configuration or creates default
pub fn load_config_file() -> SensorConfig {
    let config_dir = env::current_dir()
        .unwrap()
        .join("config")
        .join("sensor-bluetooth-grpc.toml");
    let res = confy::load_path(config_dir);
    let cfg: SensorConfig = match res {
        Ok(r) => r,
        Err(_e) => SensorConfig::default(),
    };
    cfg
}
