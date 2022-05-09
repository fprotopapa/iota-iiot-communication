#![allow(dead_code)]
use crate::config::PUBLIC_CHANNEL_ID;
use crate::recv_mqtt::receive_mqtt_messages;
use crate::req_verification::request_identity_verification;
use crate::send_mqtt::send_sensor_data;
use crate::util::{get_channel_ids, get_sensor_ids, is_factory};
use tokio::join;
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
        let mut postfix = 0;
        for channel_id in channel_ids.clone() {
            info!("State Machine: Channel Key: {}", channel_id);
            postfix += 1;
            let (rx, id) = join!(
                // Check for new MQTT Messages
                receive_mqtt_messages(&channel_id, postfix),
                // Check for Unverified Identities
                request_identity_verification(&channel_id)
            );
            match rx {
                Ok(r) => info!("{}", r),
                Err(e) => error!("{}", e),
            }
            match id {
                Ok(r) => info!("{}", r),
                Err(e) => error!("{}", e),
            }
        }
        // Publish Verified Data to Public Stream
        match publish_data(is_factory, &sensor_ids, PUBLIC_CHANNEL_ID).await {
            Ok(r) => info!("{}", r),
            Err(e) => error!("{}", e),
        }
        // Wait ...
        sleep(Duration::from_millis(10000)).await;
    }
}

async fn publish_data(
    is_factory: bool,
    sensor_ids: &Vec<String>,
    channel_id: &str,
) -> Result<String, String> {
    if is_factory {
        for sensor_id in sensor_ids {
            info!(
                "Publish Data: Channel Key: {}, Sensor ID: {}",
                channel_id, sensor_id
            );
            // Publish Verified Data to Public Stream
            match send_sensor_data(channel_id, sensor_id).await {
                Ok(r) => info!("{}", r),
                Err(e) => {
                    error!("{}", e);
                    return Err(format!("Unable to Publish Sensor Data: {}", e));
                }
            };
        }
    }
    Ok("Sensor Data Published".to_string())
}
