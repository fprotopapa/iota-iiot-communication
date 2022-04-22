#![allow(dead_code)]
use crate::recv_mqtt::receive_mqtt_messages;
use crate::req_verification::request_identity_verification;
use crate::send_mqtt::send_sensor_data;
use crate::util::{get_channel_ids, get_sensor_ids, is_factory};

use tokio::time::{sleep, Duration};

pub async fn state_machine() -> Result<(), Box<dyn std::error::Error>> {
    info!("--- state_machine() ---");
    let is_factory = is_factory();
    let channel_ids = get_channel_ids();
    let sensor_ids = if is_factory {
        get_sensor_ids()
    } else {
        Vec::new()
    };
    loop {
        for channel_id in channel_ids.clone() {
            // Check for new MQTT Messages
            match receive_mqtt_messages(&channel_id).await {
                Ok(r) => info!("{}", r),
                Err(e) => error!("{}", e),
            };
            if is_factory {
                for sensor_id in &sensor_ids {
                    // Publish Verified Data to Public Stream
                    match send_sensor_data(&channel_id, sensor_id).await {
                        Ok(r) => info!("{}", r),
                        Err(e) => error!("{}", e),
                    };
                }
            }
            // Check for Unverified Identities
            match request_identity_verification(&channel_id).await {
                Ok(r) => info!("{}", r),
                Err(e) => error!("{}", e),
            };
            // Wait ...
            sleep(Duration::from_millis(10000)).await;
        }
    }
}
