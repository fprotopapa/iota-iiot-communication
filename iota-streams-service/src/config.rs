#![allow(dead_code)]
use confy;
use serde_derive::{Deserialize, Serialize};
use std::env;
/// ENV for GRPC Socket
const ENV_GRPC_SOCKET: &str = "STREAMS_GRPC_SOCKET";
/// Default GRPC Socket
const DEFAULT_GRPC_SOCKET: &str = "0.0.0.0:50052";
/// ENV name for Node URL for Tangle communication: STREAMS_NODE_URL
pub const ENV_NODE_URL: &str = "STREAMS_NODE_URL";
/// ENV name for local Proof of work setting: STREAMS_LOCAL_POW (default: false)
pub const ENV_LOCAL_POW: &str = "STREAMS_LOCAL_POW";
/// ENV name to set password for exporting author and subscriber state
pub const ENV_STATE_PWD: &str = "STREAMS_STATE_PWD";
/// Default valuefor node URL and password
pub const DEFAULT_NODE_URL: &str = "https://chrysalis-nodes.iota.org";
/// Default value for password to export states
pub const DEFAULT_STATE_PWD: &str = "123456";
/// Default value for folder name for saving exported states
pub const EXPORT_STATE_PATH: &str = "storage";
/// Structure used to parse configuration file
/// Socket needed for GRPC server, for example \[::1]:50051
#[derive(Debug, Serialize, Deserialize)]
pub struct Grpc {
    pub socket: String,
}
/// Default implementation uses socket at \[::1]:50051, default can be set via ENVs
impl Default for Grpc {
    fn default() -> Self {
        Grpc {
            socket: env::var(ENV_GRPC_SOCKET).unwrap_or_else(|_| DEFAULT_GRPC_SOCKET.to_string()),
        }
    }
}
/// Configuration file "streams-grpc.toml" is located at ./config/
/// Function tries to load configuration or creates default
pub fn load_config_file() -> Grpc {
    let config_dir = env::current_dir()
        .unwrap()
        .join("config")
        .join("streams-grpc.toml");
    let res = confy::load_path(config_dir);
    let cfg: Grpc = match res {
        Ok(r) => r,
        Err(_e) => Grpc::default(),
    };
    cfg
}
