#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel;
extern crate diesel_codegen;
#[macro_use]
extern crate diesel_migrations;
extern crate dotenv;

mod config;
mod db_module;
mod models;
mod schema;
mod state_machine;

use db_module as db;
use sensor_grpc_adapter as adapter;
use tokio::join;

pub mod grpc_streams {
    tonic::include_proto!("iota_streams_grpc");
}
pub mod grpc_identity {
    tonic::include_proto!("iota_identity_grpc");
}
pub mod mqtt_encoder {
    tonic::include_proto!("encoder");
}
pub mod grpc_mqtt {
    tonic::include_proto!("mqtt_grpc");
}

use config::{load_config_file, DEFAULT_BUFFER_SIZE};
use state_machine::{init, receive_sensor_data, state_machine};

// Embed SQL in Binary
embed_migrations!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Migrate DB");
    init_db();
    let cfg = load_config_file();
    let addr = cfg.grpc.socket.clone();
    info!("Initialize Gateway");
    while !init().await {}
    info!("----------------------------- Start Main Program -----------------------------");
    let (service, rx) = adapter::SensorAdapterService::new(DEFAULT_BUFFER_SIZE);
    let sensor_worker = receive_sensor_data(rx);
    let gateway_worker = state_machine();
    let grpc_server = adapter::run_sensor_adapter_server(service, &addr);
    let _result = join!(sensor_worker, gateway_worker, grpc_server);

    Ok(())
}

/// Migrate Database on Start-Up
fn init_db() {
    let connection = db::establish_connection();
    let res = embedded_migrations::run(&connection);
    match res {
        Ok(_) => info!("Migration Successful"),
        Err(_) => info!("Error Running Migration"),
    }
}
