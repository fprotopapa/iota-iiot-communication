#![allow(dead_code)]
use crate::recv_mqtt::receive_mqtt_messages;
use crate::send_mqtt::send_sensor_data;

use tokio::time::{sleep, Duration};

pub async fn state_machine() -> Result<(), Box<dyn std::error::Error>> {
    info!("--- state_machine() ---");
    loop {
        // Check for new MQTT Messages
        match receive_mqtt_messages().await {
            Ok(r) => info!("{}", r),
            Err(e) => error!("{}", e),
        };
        // Check for new DB Entries (Search for unsent (MQTT and IOTA) sensor entries and process those)
        match send_sensor_data().await {
            Ok(r) => info!("{}", r),
            Err(e) => error!("{}", e),
        };
        // Check for Unverified Identities
        match check_identities().await {
            Ok(r) => info!("{}", r),
            Err(e) => error!("{}", e),
        };
        // Wait ...
        sleep(Duration::from_millis(10000)).await;
    }
}

pub async fn check_identities() -> Result<String, String> {
    Ok("".to_string())
}
