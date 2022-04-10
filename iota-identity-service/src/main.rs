use tonic::transport::Server;

mod iota_identity_module;

mod grpc_service;
use grpc_service::grpc_identity::iota_identifier_server::IotaIdentifierServer;
use grpc_service::IotaIdentityService;

mod config;
use config::load_config_file;

/// Tokio runtime and start-up code for server implementation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config_file();
    let addr = cfg.grpc.socket.parse()?;
    let service = IotaIdentityService::default();
    // Start thread
    let _grpc_server = Server::builder()
        .add_service(IotaIdentifierServer::new(service))
        .serve(addr)
        .await?;
    Ok(())
}
