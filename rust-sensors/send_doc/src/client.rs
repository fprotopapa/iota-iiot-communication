#[macro_use]
extern crate log;
use sensor_grpc_adapter as adapter;

mod config;
use config::load_config_file;

use std::time::SystemTime;
use tokio::time::{sleep, Duration};
/// Simple Client Implementation Generating Sensor Data
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Start SAP Client");
    let cfg = load_config_file();
    let mut client = adapter::connect_sensor_adapter_client(&cfg.grpc.socket).await?;
    sleep(Duration::from_millis(10000)).await;
    let doc = get_cargo_doc("WOT-PW123").await;
    info!("Send License Plate Information");
    let _ = adapter::send_sensor_data(&mut client, doc).await?;
    Ok(())
}
/// Simulate Document Generation
async fn get_cargo_doc(license_plate: &str) -> adapter::SensorData {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error Getting Time");
    adapter::SensorData {
        sensor_id: "Store-1".to_string(),
        name: "SAP-Warehouse-Logistic".to_string(),
        sensor_type: "SAP Cargo Entry".to_string(),
        value: license_plate.to_string(),
        unit: "Document".to_string(),
        timestamp: timestamp.as_secs(),
        command: "".to_string(),
    }
}

