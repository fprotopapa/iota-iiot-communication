use confy;
use serde_derive::{Deserialize, Serialize};
use std::env;
/// ENV name for Node URL for Tangle communication: STREAMS_NODE_URL
pub const ENV_NODE_URL: &str = "STREAMS_NODE_URL";
/// ENV name for local Proof of work setting: STREAMS_LOCAL_POW (default: false)
pub const ENV_LOCAL_POW: &str = "STREAMS_LOCAL_POW";
/// Default valuefor node URL and password
pub const DEFAULT_NODE_URL: &str = "https://chrysalis-nodes.iota.org";
/// Structure used to parse configuration file
/// Socket needed for GRPC server, for example \[::1]:50051
#[derive(Debug, Serialize, Deserialize)]
pub struct Streams {
    pub announcement: String,
}
/// Default implementation uses socket at \[::1]:50051, default can be set via ENVs
impl Default for Streams {
    fn default() -> Self {
        Streams {
            announcement: "".to_string(),
        }
    }
}
/// Configuration file "streams-grpc.toml" is located at ./config/
/// Function tries to load configuration or creates default
pub fn load_config_file() -> Streams {
    let config_dir = env::current_dir().unwrap().join("public.toml");
    let res = confy::load_path(config_dir);
    let cfg: Streams = match res {
        Ok(r) => r,
        Err(_e) => Streams::default(),
    };
    cfg
}
