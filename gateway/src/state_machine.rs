#![allow(dead_code)]
use crate::recv_mqtt::receive_mqtt_messages;
use crate::req_verification::request_identity_verification;
use crate::send_mqtt::send_sensor_data;
use tokio::join;
use tokio::time::{sleep, Duration};

pub async fn state_machine() -> Result<(), Box<dyn std::error::Error>> {
    info!("--- state_machine() ---");
    loop {
        
        let (rx, tx, id) = join!(
            // Check for new MQTT Messages
            receive_mqtt_messages(),
            // Check for new DB Entries (Search for unsent (MQTT and IOTA) sensor entries and process those)
            send_sensor_data(),
            // Check for Unverified Identities
            request_identity_verification()
        );
        match rx {
            Ok(r) => info!("{}", r),
            Err(e) => error!("{}", e),
        }
        match tx {
            Ok(r) => info!("{}", r),
            Err(e) => error!("{}", e),
        }
        match id {
            Ok(r) => info!("{}", r),
            Err(e) => error!("{}", e),
        }
        // Wait ...
        sleep(Duration::from_millis(10000)).await;
    }
}
