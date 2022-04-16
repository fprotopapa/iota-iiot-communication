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
mod prolog;
mod recv_mqtt;
mod req_verification;
mod schema;
mod send_mqtt;
mod state_machine;
mod util;

use db_module as db;

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

use prolog::init;
use state_machine::state_machine;
// Embed SQL in Binary
embed_migrations!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Migrate DB");
    init_db();

    info!("Initialize Client");
    while !(match init().await {
        Ok(r) => r,
        Err(e) => e,
    }) {}
    info!("----------------------------- Start Main Program -----------------------------");
    let _ = state_machine();
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
