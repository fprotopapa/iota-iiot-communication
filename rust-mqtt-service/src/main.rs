use tonic::transport::Server;

mod mqtt_module;

mod grpc_service;
use grpc_service::grpc_mqtt::mqtt_operator_server::MqttOperatorServer;
use grpc_service::MqttOperatorService;

mod config;
use config::load_config_file;

/// Tokio Runtime and Start-Up Code for Server Implementation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config_file();
    let addr = cfg.grpc.socket.parse()?;
    let service = MqttOperatorService::new(true).await;
    // Start thread
    let _grpc_server = Server::builder()
        .add_service(MqttOperatorServer::new(service))
        .serve(addr)
        .await?;
    Ok(())
}
