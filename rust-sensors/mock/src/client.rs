#[macro_use]
extern crate log;
use sensor_grpc_adapter as adapter;

mod config;
use config::load_config_file;

use rand::Rng;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};
/// Simple Client Implementation Generating Sensor Data
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Start Mock Client");
    let cfg = load_config_file();
    let mut client = adapter::connect_sensor_adapter_client(&cfg.grpc.socket).await?;
    loop {
        sleep(Duration::from_millis(cfg.mock.delay_ms)).await;
        let temp_data = sim_temperature().await;
        match adapter::send_sensor_data(&mut client, temp_data).await {
            Ok(r) => info!("Response T-Sensor: {}", r.status),
            Err(e) => error!("Unable to Send Data: {}", e),
        };
        sleep(Duration::from_millis(cfg.mock.delay_ms)).await;
        let humid_data = sim_humidity().await;
        match adapter::send_sensor_data(&mut client, humid_data).await {
            Ok(r) => info!("Response H-Sensor: {}", r.status),
            Err(e) => error!("Unable to Send Data: {}", e),
        };
    }
}
/// Simulate Temperature Sensor
async fn sim_temperature() -> adapter::SensorData {
    let mut rng = rand::thread_rng();
    let temperature = rng.gen_range(40.0..80.0);
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error Getting Time");
    adapter::SensorData {
        sensor_id: "12".to_string(),
        name: "MPC 7812".to_string(),
        sensor_type: "Temperature".to_string(),
        value: temperature.to_string(),
        unit: "C".to_string(),
        timestamp: timestamp.as_secs(),
        command: "".to_string(),
    }
}
/// Simulate Humitidy Sensor
async fn sim_humidity() -> adapter::SensorData {
    let mut rng = rand::thread_rng();
    let humidity = rng.gen_range(60..99);
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error Getting Time");
    adapter::SensorData {
        sensor_id: "5".to_string(),
        name: "DHT 15".to_string(),
        sensor_type: "Humidity".to_string(),
        value: humidity.to_string(),
        unit: "%".to_string(),
        timestamp: timestamp.as_secs(),
        command: "".to_string(),
    }
}
