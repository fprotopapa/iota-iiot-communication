#[macro_use]
extern crate log;

use tokio::join;
use tokio::sync::mpsc;
use tonic::transport::Server;

mod grpc_service;
use grpc_service::grpc_streams::iota_streamer_server::IotaStreamerServer;
use grpc_service::{IotaStreamsService, QueueElem};

mod streams_service;
use streams_service::streams_state_machine;

mod iota_streams_module;
pub use crate::iota_streams_module::streams_author;

mod config;
pub use crate::config::load_config_file;

mod msg_util;
/// Tokio runtime and start-up code for server implementation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cfg = load_config_file();
    let addr = cfg.socket.parse()?;
    info!("Start IOTA Streams Service");
    // Communication channel used btw streams_state_machine and GRPC server
    let (tx, rx) = mpsc::channel::<QueueElem>(32);
    let service = IotaStreamsService::new(tx.clone());
    // Start threads
    let streams_worker = streams_state_machine(rx);
    let grpc_server = Server::builder()
        .add_service(IotaStreamerServer::new(service))
        .serve(addr);
    let _result = join!(streams_worker, grpc_server);
    Ok(())
}
