#![allow(dead_code)]

use serde_json::json;
use std::env;

use crate::config::{
    ENV_CHANNEL_KEY, ENV_DEVICE_ID, ENV_THING_KEY, MQTT_SOCKET, STREAMS_SOCKET, TOPIC_SENSOR_VALUE,
};
use crate::db_module as db;
use crate::grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use crate::grpc_streams::iota_streamer_client::IotaStreamerClient;
use crate::grpc_streams::IotaStreamsSendMessageRequest;
use crate::mqtt_encoder as enc;
use crate::recv_mqtt::receive_mqtt_messages;
use crate::util::{send_mqtt_message, serialize_msg};
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
        // Wait ...
        sleep(Duration::from_millis(10000)).await;
    }
}

pub async fn send_sensor_data() -> Result<String, String> {
    info!("--- send_sensor_data() ---");
    let author_id = env::var(ENV_DEVICE_ID).expect("ENV for Author ID not Found");
    info!("ENV: {} = {}", ENV_DEVICE_ID, &author_id);
    let channel_key = env::var(ENV_CHANNEL_KEY).expect("ENV for Channel Key not Found");
    info!("ENV: {} = {}", ENV_CHANNEL_KEY, &channel_key);
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    // Connect to Database
    let db_client = db::establish_connection();
    // Connect to MQTT Service
    let mut mqtt_client = match MqttOperatorClient::connect(format!("http://{}", MQTT_SOCKET)).await
    {
        Ok(res) => {
            info!("Connected to MQTT Service");
            res
        }
        Err(e) => return Err(format!("Error Connecting to MQTT-Service: {}", e)),
    };
    // Connect to IOTA Streams Service
    let mut stream_client =
        match IotaStreamerClient::connect(format!("http://{}", STREAMS_SOCKET)).await {
            Ok(res) => {
                info!("Connected to IOTA Streams Service");
                res
            }
            Err(e) => return Err(format!("Error Connecting to Streams Service: {}", e)),
        };
    let val_mqtt = match db::select_sensor_entry(&db_client, "mqtt", false, 0) {
        Ok(res) => {
            info!("Sensor Entries Selected with MQTT: false");
            res
        }
        Err(_) => return Err(format!("Unable to Select Sensor Entries with MQTT: false")),
    };
    for val in val_mqtt {
        let sensor = match db::select_sensor(&db_client, val.sensor_id) {
            Ok(res) => {
                info!("Sensor Selected with ID: {}", val.sensor_id);
                res
            }
            Err(_) => {
                return Err(format!(
                    "Unable to Select Sensor with ID: {}",
                    val.sensor_id
                ))
            }
        };
        let sensor_type = match db::select_sensor_type_by_id(&db_client, sensor.sensor_types_id) {
            Ok(res) => {
                info!("Sensor Type Selected with ID: {}", sensor.sensor_types_id);
                res
            }
            Err(_) => {
                return Err(format!(
                    "Unable to Select Sensor with ID: {}",
                    sensor.sensor_types_id
                ))
            }
        };
        let payload = serialize_msg(&enc::Sensor {
            sensor_id: sensor.sensor_id,
            name: match sensor.sensor_name {
                Some(r) => r,
                None => "".to_string(),
            },
            typ: sensor_type.description,
            value: val.sensor_value,
            unit: match sensor_type.unit {
                Some(r) => r,
                None => "".to_string(),
            },
            timestamp: val.sensor_time,
        });
        match send_mqtt_message(&mut mqtt_client, payload, TOPIC_SENSOR_VALUE).await {
            Ok(_) => match db::update_sensor_entry(&db_client, val.id, "mqtt", true) {
                Ok(_) => info!("Sensor Entry Updated to mqtt = true"),
                Err(_) => return Err(format!("Unable to Update Sensor Entry to mqtt = true")),
            },
            Err(e) => error!(
                "Unable to Send MQTT Message to Topic: sensors, Error: {}",
                e
            ),
        };
    }
    // Send Data to Tangle
    // Get Channel ID
    let channel = match db::select_channel(&db_client, &channel_key) {
        Ok(res) => {
            info!("Channel Selected with Key: {}", &channel_key);
            res
        }
        Err(_) => {
            return Err(format!(
                "Unable to Select Channel with Key: {}",
                &channel_key
            ))
        }
    };
    // Get Message Link
    let stream_entry = match db::select_stream(&db_client, channel.id) {
        Ok(res) => {
            info!("Stream Entry Selected with Channel ID: {}", channel.id);
            res
        }
        Err(_) => {
            return Err(format!(
                "Unable to Select Stream Entry with Channel ID: {}",
                channel.id
            ))
        }
    };
    // Check if Keyloads are sent,
    match stream_entry.key_link {
        Some(r) => {
            if r.is_empty() {
                return Ok("No IOTA Streams Connection Established (Keyload Missing)".to_string());
            }
        }
        None => return Ok("No IOTA Streams Connection Established (Keyload Missing)".to_string()),
    }
    // Check for all subscribers?
    // Check if IOTA values available?
    let val_iota = match db::select_sensor_entry(&db_client, "iota", false, 0) {
        Ok(res) => {
            info!("Sensor Entries Selected with IOTA: false");
            res
        }
        Err(_) => return Err(format!("Unable to Select Sensor Entries with IOTA: false")),
    };
    // Get Thing ID
    let thing = match db::select_thing(&db_client, &thing_key) {
        Ok(res) => {
            info!("Thing Selected with Key: {}", &thing_key);
            res
        }
        Err(_) => return Err(format!("Unable to Select Thing with Key: {}", &thing_key)),
    };
    // Get own DID
    let identity = match db::select_identification(&db_client, thing.id) {
        Ok(res) => {
            info!("Thing Identity Selected with ID: {}", thing.id);
            res
        }
        Err(_) => {
            return Err(format!(
                "Unable to Select Thing Identity with ID: {}",
                thing.id
            ))
        }
    };

    let mut msg_link = match stream_entry.msg_link {
        Some(r) => {
            info!("IOTA Streams Message Link: {}", &r);
            r
        }
        None => {
            return Err(format!(
                "No IOTA Streams Connection Established (Message Link Missing)"
            ))
        }
    };
    for val in val_iota {
        let sensor = match db::select_sensor(&db_client, val.sensor_id) {
            Ok(res) => {
                info!("Sensor Selected with ID: {}", val.sensor_id);
                res
            }
            Err(_) => {
                return Err(format!(
                    "Unable to Select Sensor with ID: {}",
                    val.sensor_id
                ))
            }
        };
        let sensor_type = match db::select_sensor_type_by_id(&db_client, sensor.sensor_types_id) {
            Ok(res) => {
                info!("Selected Sensor Type with ID: {}", sensor.sensor_types_id);
                res
            }
            Err(_) => {
                return Err(format!(
                    "Unable to Select Sensor Type with ID: {}",
                    sensor.sensor_types_id
                ))
            }
        };
        // Make Payload
        let payload = json!({
            "did": identity.did,
            "verifiable_credential": identity.vc,
            "sensor_id": sensor.sensor_id,
            "sensor_name": sensor.sensor_name,
            "sensor_type": sensor_type.description,
            "value": val.sensor_value,
            "unit": sensor_type.unit,
            "timestamp": val.sensor_time,
        })
        .to_string();
        info!("Send Message to Tangle");
        // Send IOTA Streams Message
        let response = match stream_client
            .send_message(IotaStreamsSendMessageRequest {
                id: author_id.clone(),
                msg_type: 5, // SendMessage
                message_link: msg_link,
                message_length: payload.len() as u32,
                message: payload,
            })
            .await
        {
            Ok(res) => {
                // Update Message Link
                let response = res.into_inner();
                match db::update_stream(&db_client, channel.id, "msg_link", &response.link) {
                    Ok(_) => info!("Update Message Link to {}", &response.link),
                    Err(_) => {
                        return Err(format!(
                            "Unable to Update Message Link Entry to {}",
                            &response.link
                        ))
                    }
                };
                // Update Sensor Entry
                match db::update_sensor_entry(&db_client, val.id, "iota", true) {
                    Ok(_) => info!("Update Sensor Entry, set IOTA = true"),
                    Err(_) => {
                        return Err(format!("Unable to Update Sensor Entry, set IOTA = true"))
                    }
                };
                response
            }
            Err(_) => return Err(format!("Unable to Send Message to Tangle")),
        };
        msg_link = response.link;
    }
    Ok("Exit with Success: send_sensor_data()".to_string())
}
