use chrono::{DateTime, NaiveDateTime, Utc};
use serde_json::json;
use std::env;

use crate::config::{ENV_DEVICE_ID, ENV_LATEST_TIMESTAMP, ENV_THING_KEY};
use crate::db_module as db;
use crate::grpc_streams::iota_streamer_client::IotaStreamerClient;
use crate::grpc_streams::{IotaStreamsReply, IotaStreamsSendMessageRequest};
use crate::models::{Sensor, SensorData, SensorType, Stream};
use crate::util::{
    connect_streams, get_channel, get_identification, get_thing, parse_env, update_streams_entry,
};

pub async fn send_sensor_data(channel_key: &str, sensor_id: i32) -> Result<String, String> {
    info!("--- send_sensor_data() ---");
    let author_id = env::var(ENV_DEVICE_ID).expect("ENV for Author ID not Found");
    info!("ENV: {} = {}", ENV_DEVICE_ID, &author_id);
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    // Connect to IOTA Streams Service
    let mut stream_client = connect_streams().await?;
    // Get Latest Timestamp
    let timestamp = parse_env::<i64>(ENV_LATEST_TIMESTAMP);
    // Connect to Database
    let db_client = db::establish_connection();
    let db_entries = match db::select_sensor_entry_for_tangle(&db_client, sensor_id, timestamp) {
        Ok(r) => r,
        Err(_) => return Err("Unable to Select Sensor Entries".to_string()),
    };
    // Send Data to Tangle
    // Get Channel ID
    let channel = get_channel(&db_client, channel_key)?;
    // Get Message Link
    let stream_entry = get_streams(&db_client, channel.id)?;
    // Check if Keyloads are sent,
    let key_link = match stream_entry.key_link {
        Some(r) => {
            if r.is_empty() {
                return Ok("No IOTA Streams Connection Established (Keyload Missing)".to_string());
            }
            r
        }
        None => return Ok("No IOTA Streams Connection Established (Keyload Missing)".to_string()),
    };
    // Get Thing ID
    let thing = get_thing(&db_client, &thing_key)?;
    // Get own DID
    let identity = get_identification(&db_client, thing.id)?;

    let mut msg_link = match stream_entry.msg_link {
        Some(r) => {
            info!("IOTA Streams Message Link: {}", &r);
            r
        }
        None => {
            info!("IOTA Streams Message Link (use Key Link): {}", &key_link);
            key_link
        }
    };
    for val in db_entries {
        let sensor = get_sensor(&db_client, val.sensor_id)?;
        let sensor_type = get_sensor_type(&db_client, sensor.sensor_types_id)?;
        // Make Payload
        let payload = json!({
            "did": identity.did,
            "verifiable_credential": identity.vc,
            "sensor_id": sensor.sensor_id,
            "sensor_name": sensor.sensor_name,
            "sensor_type": sensor_type.description,
            "value": val.sensor_value,
            "unit": sensor_type.unit,
            "timestamp": unix_to_utc(val.sensor_time).to_string(),
        })
        .to_string();
        info!("Send Message to Tangle");
        // Send IOTA Streams Message
        let response =
            send_message_to_tangle(&mut stream_client, &msg_link, &payload, &author_id).await?;
        update_streams_entry(&db_client, &response.link, 0, "msg_link", channel.id)?;
        env::set_var(ENV_LATEST_TIMESTAMP, val.sensor_time.to_string());
        //info!("Saved in ENV: {}", ENV_LATEST_TIMESTAMP);
        msg_link = response.link;
    }
    Ok("Exit with Success: send_sensor_data()".to_string())
}

fn unix_to_utc(timestamp: i64) -> DateTime<Utc> {
    let naive =
        NaiveDateTime::from_timestamp_opt(timestamp / 1000, (timestamp % 1000) as u32 * 1_000_000)
            .unwrap();
    DateTime::<Utc>::from_utc(naive, Utc)
}

async fn send_message_to_tangle(
    stream_client: &mut IotaStreamerClient<tonic::transport::Channel>,
    msg_link: &str,
    payload: &str,
    author_id: &str,
) -> Result<IotaStreamsReply, String> {
    match stream_client
        .send_message(IotaStreamsSendMessageRequest {
            id: author_id.to_string(),
            msg_type: 5, // SendMessage
            message_link: msg_link.to_string(),
            message_length: payload.len() as u32,
            message: payload.to_string(),
        })
        .await
    {
        Ok(res) => {
            // Update Message Link
            info!("Send Message to Tangle");
            let response = res.into_inner();
            return Ok(response);
        }
        Err(_) => return Err(format!("Unable to Send Message to Tangle")),
    };
}

fn get_sensor_data(
    db_client: &diesel::SqliteConnection,
    query: &str,
    is_true: bool,
) -> Result<Vec<SensorData>, String> {
    match db::select_sensor_entry(db_client, query, is_true, 0) {
        Ok(res) => {
            info!("Sensor Entries Selected with {}: {}", query, is_true);
            return Ok(res);
        }
        Err(_) => {
            return Err(format!(
                "Unable to Select Sensor Entries with {}: {}",
                query, is_true
            ))
        }
    };
}

fn get_sensor(db_client: &diesel::SqliteConnection, sensor_id: i32) -> Result<Sensor, String> {
    match db::select_sensor(&db_client, sensor_id) {
        Ok(res) => {
            info!("Sensor Selected with ID: {}", sensor_id);
            return Ok(res);
        }
        Err(_) => return Err(format!("Unable to Select Sensor with ID: {}", sensor_id)),
    };
}

fn get_sensor_type(
    db_client: &diesel::SqliteConnection,
    type_id: i32,
) -> Result<SensorType, String> {
    match db::select_sensor_type_by_id(db_client, type_id) {
        Ok(res) => {
            info!("Sensor Type Selected with ID: {}", type_id);
            return Ok(res);
        }
        Err(_) => return Err(format!("Unable to Select Sensor with ID: {}", type_id)),
    };
}

fn update_sensor_entry(
    db_client: &diesel::SqliteConnection,
    entry_id: i32,
    query: &str,
    is_true: bool,
) -> Result<(), String> {
    match db::update_sensor_entry(db_client, entry_id, query, is_true) {
        Ok(_) => info!("Sensor Entry Updated to {} = {}", query, is_true),
        Err(_) => {
            return Err(format!(
                "Unable to Update Sensor Entry to {} = {}",
                query, is_true
            ))
        }
    };
    Ok(())
}

fn get_streams(db_client: &diesel::SqliteConnection, channel_id: i32) -> Result<Stream, String> {
    match db::select_stream(&db_client, channel_id) {
        Ok(res) => {
            info!("Stream Entry Selected with Channel ID: {}", channel_id);
            return Ok(res);
        }
        Err(_) => {
            return Err(format!(
                "Unable to Select Stream Entry with Channel ID: {}",
                channel_id
            ))
        }
    };
}
