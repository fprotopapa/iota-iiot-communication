#![allow(dead_code)]
use prost::Message;

use std::env;
use std::io::Cursor;

use crate::config::{
    ENV_THING_KEY, TOPIC_COMMAND, TOPIC_DID, TOPIC_IDENTITY, TOPIC_SENSOR_VALUE, TOPIC_SETTING,
    TOPIC_STREAM,
};
use crate::db_module as db;
use crate::grpc_identity::iota_identifier_client::IotaIdentifierClient;
use crate::grpc_identity::IotaIdentityRequest;
use crate::grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use crate::grpc_mqtt::{MqttMsgsReply, MqttRequest};
use crate::grpc_streams::IotaStreamsRequest;
use crate::models::{Identity, Sensor, SensorData, SensorType};
use crate::mqtt_encoder as enc;
use crate::util::{
    connect_identity, connect_mqtt, connect_streams, get_channel, get_identification, get_thing,
    helper_send_mqtt, serialize_msg, update_streams_entry,
};
use serde_derive::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone)]
struct MessageFromJson {
    did: String,
    verifiable_credential: String,
    sensor_id: String,
    sensor_name: String,
    sensor_type: String,
    value: String,
    unit: String,
    timestamp: i64,
}

pub async fn receive_mqtt_messages(channel_key: &str) -> Result<String, String> {
    info!("--- receive_mqtt_messages() ---");
    let mut mqtt_client = connect_mqtt().await?;
    info!("Receive MQTT Messages ...");
    let mut response = receive_messages(&mut mqtt_client).await?;
    for (payload, topic) in response.messages.iter_mut().zip(response.topics) {
        let result = match topic.as_str() {
            TOPIC_DID => mqtt_identity(payload.to_vec(), channel_key).await,
            TOPIC_STREAM => mqtt_streams(payload.to_vec(), channel_key).await,
            TOPIC_SETTING => mqtt_settings(payload.to_vec()).await,
            TOPIC_COMMAND => mqtt_command(payload.to_vec()).await,
            TOPIC_IDENTITY => mqtt_first_verification(payload.to_vec()).await,
            TOPIC_SENSOR_VALUE => mqtt_save_sensor_data(payload.to_vec(), channel_key).await,
            e => Err(format!("Topic {} not Found", e)),
        };
        match result {
            Ok(_) => (),
            Err(e) => error!("{}", e),
        }
    }
    Ok("Exit with Success: receive_mqtt_messages()".to_string())
}

pub async fn mqtt_save_sensor_data(payload: Vec<u8>, channel_key: &str) -> Result<u32, String> {
    info!("--- mqtt_save_sensor_data() ---");
    // Decode Payload
    let msg = match enc::Sensor::decode(&mut Cursor::new(payload)) {
        Ok(res) => res,
        Err(e) => return Err(format!("Unable to Decode Payload: {}", e)),
    };
    let db_client = db::establish_connection();
    let channel = get_channel(&db_client, channel_key)?;
    save_mqtt_sensor_data(&db_client, channel.id, msg)?;
    let mut streams_client = connect_streams().await?;
    // ToDo: Check for keyload
    let msgs = match streams_client
        .receive_messages(IotaStreamsRequest {
            id: channel_key.to_string(),
            link: "".to_string(),
            msg_type: 6, // ReceiveMessages
        })
        .await
    {
        Ok(r) => r.into_inner(),
        Err(e) => return Err(format!("Unable to Receive Messages: {}", e)),
    };
    for msg in msgs.messages {
        save_iota_sensor_data(&db_client, &msg, channel.id).await?;
    }
    Ok(0)
}

pub async fn mqtt_first_verification(payload: Vec<u8>) -> Result<u32, String> {
    info!("--- mqtt_first_verification() ---");
    // Decode Payload
    let msg = match enc::Did::decode(&mut Cursor::new(payload)) {
        Ok(res) => res,
        Err(e) => return Err(format!("Unable to Decode Payload: {}", e)),
    };
    let mut identity_client = connect_identity().await?;
    let db_client = db::establish_connection();
    // Verify Identity
    let response = match identity_client
        .verify_identity(tonic::Request::new(IotaIdentityRequest {
            did: msg.did.clone(),
            challenge: msg.challenge,
            verifiable_credential: msg.vc,
        }))
        .await
    {
        Ok(res) => res.into_inner(),
        Err(e) => return Err(format!("Unable to Verify Identity: {}", e)),
    };
    let is_verified = if response.code == 0 { true } else { false };
    let _ = get_identity(&db_client, &msg.did)?;
    update_identity(&db_client, &msg.did, is_verified)?;
    Ok(0)
}

pub async fn mqtt_streams(payload: Vec<u8>, channel_key: &str) -> Result<u32, String> {
    info!("--- mqtt_streams() ---");
    let sub_id = channel_key; //env::var(ENV_DEVICE_ID).expect("ENV for Author ID not Found");
                              //info!("ENV: {} = {}", ENV_DEVICE_ID, &sub_id);
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    // Decode Payload
    let msg = match enc::Streams::decode(&mut Cursor::new(payload)) {
        Ok(res) => res,
        Err(e) => return Err(format!("Unable to Decode Payload: {}", e)),
    };

    // Connect to Database
    let db_client = db::establish_connection();
    // Verify that Message wasn't sent from this Thing
    // Get Thing ID
    let thing = get_thing(&db_client, &thing_key)?;
    // Get own DID
    let identity = get_identification(&db_client, thing.id)?;
    if identity.did.eq(&msg.did) {
        return Err(format!("Received Own Message, DID: {}", &msg.did));
    }
    // Check for Unverifiable Identities
    let msg_identity = get_identity(&db_client, &msg.did)?;
    let is_verified = match msg_identity.verified {
        Some(r) => r,
        None => false,
    };

    if !msg.announcement_link.is_empty() && is_verified {
        make_subscriber(
            &db_client,
            &sub_id,
            &msg.announcement_link,
            channel_key,
            &thing_key,
        )
        .await?;
    } else if !msg.keyload_link.is_empty() && is_verified {
        add_keyload(&db_client, &sub_id, &msg.keyload_link, channel_key).await?;
    }
    info!("Streams Message Processed");
    Ok(0)
}

pub async fn mqtt_identity(payload: Vec<u8>, channel_id: &str) -> Result<u32, String> {
    info!("--- mqtt_identity() ---");
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    // Connect to Identity Service
    let mut identity_client = connect_identity().await?;
    // Connect to MQTT Service
    let mut mqtt_client = connect_mqtt().await?;
    // Connect to Database
    let db_client = db::establish_connection();
    // Decode Message
    let msg = match enc::Did::decode(&mut Cursor::new(payload)) {
        Ok(res) => res,
        Err(e) => return Err(format!("Decoding Error: {}", e)),
    };
    // Get DID
    let thing = get_thing(&db_client, &thing_key)?;
    let identity = get_identification(&db_client, thing.id)?;
    // Check if DID in Message is same as DID of this thing
    let is_thing = if identity.did.eq(&msg.did) {
        true
    } else {
        info!("DID Unequal to Gateway DID");
        false
    };
    // If Thing is requested to proof identity and requested DID is same
    // Sign VC with challenge
    if msg.proof && is_thing {
        info!("Proof Gateway Identity");
        proof_identity(&mut identity_client, &mut mqtt_client, msg, channel_id).await?;
    // Thing should verify received DID
    } else if !msg.proof && !is_thing {
        info!("Verify Participant's Identity");
        verify_identity(&mut identity_client, &db_client, msg).await?;
    }
    Ok(0)
}

pub fn save_mqtt_sensor_data(
    db_client: &diesel::SqliteConnection,
    channel_id: i32,
    msg: enc::Sensor,
) -> Result<u32, String> {
    match db::create_sensor_type(&db_client, &msg.typ, &msg.unit) {
        Ok(_) => info!("Sensor Type Entry Created for Sensor: {}", &msg.typ),
        Err(e) => {
            error!("Error Creating Sensor Type Entry for : {}", e)
        }
    };
    let sensor_type = match db::select_sensor_type_by_desc(&db_client, &msg.typ) {
        Ok(res) => {
            info!("Sensor Type Entry Selected");
            res
        }
        Err(e) => {
            return Err(format!("Unable to Select Sensor Type Entry: {}", e));
        }
    };
    match db::create_sensor(
        &db_client,
        db::SensorEntry {
            channel_id: channel_id,
            sensor_types_id: sensor_type.id,
            sensor_id: msg.sensor_id.clone(),
            sensor_name: msg.name.clone(),
        },
    ) {
        Ok(_) => info!(
            "Sensor Entry Created for ID: {}, Name: {}",
            &msg.sensor_id, &msg.name
        ),
        Err(e) => error!(
            "Error Creating Sensor Entry for ID: {}, Name: {}: {}",
            &msg.sensor_id, &msg.name, e
        ),
    };
    let sensor = match db::select_sensor_by_name(&db_client, &msg.sensor_id) {
        Ok(r) => {
            info!("Sensor Entry Selected");
            r
        }
        Err(e) => return Err(format!("Unable to Select Sensor Entry: {}", e)),
    };
    let data_lst =
        match db::select_sensor_entry_by_time_and_id(&db_client, sensor.id, msg.timestamp) {
            Ok(r) => {
                info!("Found Sensor Data Entry");
                r
            }
            Err(_) => {
                info!("No Entry Found, Making New Data Entry");
                make_sensor_data_entry(
                    &db_client,
                    sensor.id,
                    &msg.value,
                    msg.timestamp,
                    true,
                    false,
                )?;
                return Ok(0);
            }
        };
    for data in data_lst {
        let id = data.id;
        if compare_mqtt_to_db(data, msg.clone(), sensor_type.clone(), sensor.clone()) {
            match db::update_sensor_entry(&db_client, id, "verified", true) {
                Ok(_) => {
                    info!("Data Entry Verified");
                    return Ok(0);
                }
                Err(e) => return Err(format!("Unable to Verify Data Entry: {}", e)),
            };
        } else {
            error!("Unable to Verify Data Entry");
        }
    }
    Ok(0)
}

async fn save_iota_sensor_data(
    db_client: &diesel::SqliteConnection,
    message: &str,
    channel_id: i32,
) -> Result<u32, String> {
    let msg: MessageFromJson = match serde_json::from_str(message) {
        Ok(r) => r,
        Err(e) => {
            return Err(format!("Unable to Parse JSON to Message: {}", e));
        }
    };
    match db::create_sensor_type(&db_client, &msg.sensor_type, &msg.unit) {
        Ok(_) => info!("Sensor Type Entry Created for Sensor: {}", &msg.sensor_type),
        Err(e) => {
            error!("Error Creating Sensor Type Entry for : {}", e)
        }
    };
    let sensor_type = match db::select_sensor_type_by_desc(&db_client, &msg.sensor_type) {
        Ok(res) => {
            info!("Sensor Type Entry Selected");
            res
        }
        Err(e) => {
            return Err(format!("Unable to Select Sensor Type Entry: {}", e));
        }
    };
    match db::create_sensor(
        &db_client,
        db::SensorEntry {
            channel_id: channel_id,
            sensor_types_id: sensor_type.id,
            sensor_id: msg.sensor_id.clone(),
            sensor_name: msg.sensor_name.clone(),
        },
    ) {
        Ok(_) => info!(
            "Sensor Entry Created for ID: {}, Name: {}",
            &msg.sensor_id, &msg.sensor_name
        ),
        Err(e) => error!(
            "Error Creating Sensor Entry for ID: {}, Name: {}: {}",
            &msg.sensor_id, &msg.sensor_name, e
        ),
    };
    let sensor = match db::select_sensor_by_name(&db_client, &msg.sensor_id) {
        Ok(r) => {
            info!("Sensor Entry Selected");
            r
        }
        Err(e) => return Err(format!("Unable to Select Sensor Entry: {}", e)),
    };
    let data_lst =
        match db::select_sensor_entry_by_time_and_id(&db_client, sensor.id, msg.timestamp) {
            Ok(r) => {
                info!("Found Sensor Data Entry");
                r
            }
            Err(_) => {
                info!("No Entry Found, Making New Data Entry");
                make_sensor_data_entry(
                    &db_client,
                    sensor.id,
                    &msg.value,
                    msg.timestamp,
                    false,
                    true,
                )?;
                return Ok(0);
            }
        };
    for data in data_lst {
        let id = data.id;
        if compare_iota_to_db(data, msg.clone(), sensor_type.clone(), sensor.clone()) {
            match db::update_sensor_entry(&db_client, id, "verified", true) {
                Ok(_) => {
                    info!("Data Entry Verified");
                    return Ok(0);
                }
                Err(e) => return Err(format!("Unable to Verify Data Entry: {}", e)),
            };
        } else {
            error!("Unable to Verify Data Entry");
        }
    }
    Ok(0)
}

fn compare_iota_to_db(
    db_entry: SensorData,
    msg: MessageFromJson,
    sensor_type: SensorType,
    sensor: Sensor,
) -> bool {
    let unit = match sensor_type.unit {
        Some(r) => r,
        None => "".to_string(),
    };
    let name = match sensor.sensor_name {
        Some(r) => r,
        None => "".to_string(),
    };
    if msg.sensor_id.eq(&sensor.sensor_id)
        && msg.sensor_name.eq(&name)
        && msg.sensor_type.eq(&sensor_type.description)
        && msg.unit.eq(&unit)
        && msg.value.eq(&db_entry.sensor_value)
        && msg.timestamp.eq(&db_entry.sensor_time)
    {
        true
    } else {
        false
    }
}

fn compare_mqtt_to_db(
    db_entry: SensorData,
    msg: enc::Sensor,
    sensor_type: SensorType,
    sensor: Sensor,
) -> bool {
    let unit = match sensor_type.unit {
        Some(r) => r,
        None => "".to_string(),
    };
    let name = match sensor.sensor_name {
        Some(r) => r,
        None => "".to_string(),
    };
    if msg.sensor_id.eq(&sensor.sensor_id)
        && msg.name.eq(&name)
        && msg.typ.eq(&sensor_type.description)
        && msg.unit.eq(&unit)
        && msg.value.eq(&db_entry.sensor_value)
        && msg.timestamp.eq(&db_entry.sensor_time)
    {
        true
    } else {
        false
    }
}

fn make_sensor_data_entry(
    db_client: &diesel::SqliteConnection,
    sensor_id: i32,
    value: &str,
    time: i64,
    is_mqtt: bool,
    is_iota: bool,
) -> Result<u32, String> {
    match db::create_sensor_data(
        db_client,
        db::SensorDataEntry {
            sensor_id: sensor_id,
            sensor_value: value.to_string(),
            sensor_time: time,
            mqtt: is_mqtt,
            iota: is_iota,
            verified: false,
        },
    ) {
        Ok(_) => {
            info!("New Sensor Data Entry Created");
            return Ok(0);
        }
        Err(e) => return Err(format!("Unable to Create Sensor Data Entry: {}", e)),
    };
}

async fn verify_identity(
    identity_client: &mut IotaIdentifierClient<tonic::transport::Channel>,
    db_client: &diesel::SqliteConnection,
    identity: enc::Did,
) -> Result<u32, String> {
    let response = match identity_client
        .verify_identity(tonic::Request::new(IotaIdentityRequest {
            did: identity.did,
            challenge: identity.challenge,
            verifiable_credential: identity.vc,
        }))
        .await
    {
        Ok(res) => {
            let response = res.into_inner();
            response
        }
        Err(e) => return Err(format!("Unable to Verify Identity: {}", e)),
    };
    // Check if Verification was a success, GRPC Call returns Status = "Verified"
    if response.code == 0 {
        // Save Answer to DB
        match get_identity(&db_client, &response.did) {
            Ok(_) => {
                update_identity(&db_client, &response.did, true)?;
            }
            Err(_) => {
                make_identity(&db_client, &response.did)?;
                update_identity(&db_client, &response.did, true)?;
            }
        };
    }
    return Ok(0);
}

pub async fn mqtt_settings(payload: Vec<u8>) -> Result<u32, String> {
    info!("--- mqtt_settings() ---");
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    let msg = match enc::Setting::decode(&mut Cursor::new(payload)) {
        Ok(res) => res,
        Err(e) => return Err(format!("Error Decoding Message: {}", e)),
    };
    let db_client = db::establish_connection();
    if msg.pk_timestamp != 0 {
        let thing = match db::select_thing(&db_client, &thing_key) {
            Ok(res) => res,
            Err(e) => return Err(format!("Thing Not Found: {}", e)),
        };
        match db::update_configuration(&db_client, thing.id, "pk_timestamp", "", msg.pk_timestamp) {
            Ok(_) => {
                let path = Path::new(".").join("cert").join("ca.crt");
                match fs::write(path, msg.pk) {
                    Ok(_) => return Ok(0),
                    Err(e) => return Err(format!("Error Creating ca.crt: {}", e)),
                };
            }
            Err(e) => return Err(format!("Error Updating Configuration: {}", e)),
        };
    }
    Ok(0)
}

pub async fn mqtt_command(payload: Vec<u8>) -> Result<u32, String> {
    // Not Implemented
    info!("--- mqtt_command() --- not Implemented");
    let _msg = enc::Command::decode(&mut Cursor::new(payload));
    Ok(0)
}

pub async fn add_keyload(
    db_client: &diesel::SqliteConnection,
    device_id: &str,
    key_link: &str,
    channel_key: &str,
) -> Result<u32, String> {
    // Connect to IOTA Streams Service
    let mut stream_client = connect_streams().await?;
    // Add Keyload
    let msg = IotaStreamsRequest {
        id: device_id.to_string(),
        msg_type: 4,
        link: key_link.to_string(),
    };
    match stream_client
        .receive_keyload(tonic::Request::new(msg))
        .await
    {
        Ok(res) => {
            let response = res.into_inner();
            if response.code != 0 {
                return Err("Unable to Add Keyload to Subscriber".to_string());
            }
        }
        Err(e) => return Err(format!("Unable to Add Keyload to Subscriber: {}", e)),
    };
    // Save Sub Link
    let channel = get_channel(&db_client, &channel_key)?;
    update_streams_entry(db_client, key_link, 0, "keyload", channel.id)?;
    Ok(0)
}

pub async fn make_subscriber(
    db_client: &diesel::SqliteConnection,
    device_id: &str,
    ann_link: &str,
    channel_key: &str,
    thing_key: &str,
) -> Result<u32, String> {
    // Connect to MQTT Service
    let mut mqtt_client = connect_mqtt().await?;
    // Connect to IOTA Streams Service
    let mut stream_client = connect_streams().await?;
    // // Save Ann Link
    let channel = get_channel(&db_client, &channel_key)?;
    update_streams_entry(db_client, ann_link, 0, "announcement", channel.id)?;
    // Create Subscriber
    let msg = IotaStreamsRequest {
        id: device_id.to_string(),
        msg_type: 2,
        link: ann_link.to_string(),
    };
    let sublink = match stream_client
        .create_new_subscriber(tonic::Request::new(msg))
        .await
    {
        Ok(res) => {
            let response = res.into_inner();
            if response.code == 0 {
                response.link
            } else {
                return Err("Unable to Create Subscriber".to_string());
            }
        }
        Err(e) => return Err(format!("Unable to Create Subscriber: {}", e)),
    };
    // Get Thing ID
    let thing = get_thing(&db_client, &thing_key)?;
    // Get own DID
    let identity = get_identification(&db_client, thing.id)?;
    // Send Sub Link over MQTT
    let payload = serialize_msg(&enc::Streams {
        announcement_link: "".to_string(),
        subscription_link: sublink.clone(),
        keyload_link: "".to_string(),
        did: identity.did,
        vc: match identity.vc {
            Some(r) => r,
            None => "".to_string(),
        },
    });
    info!("Send Subscription Link over MQTT");
    helper_send_mqtt(&mut mqtt_client, payload, TOPIC_STREAM, channel_key).await?;
    // Save Sub Link
    update_streams_entry(db_client, &sublink, 0, "subscription", channel.id)?;
    Ok(0)
}

async fn proof_identity(
    identity_client: &mut IotaIdentifierClient<tonic::transport::Channel>,
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
    identity: enc::Did,
    channel_id: &str,
) -> Result<(), String> {
    match identity_client
        .proof_identity(tonic::Request::new(IotaIdentityRequest {
            did: identity.did,
            challenge: identity.challenge,
            verifiable_credential: identity.vc,
        }))
        .await
    {
        Ok(res) => {
            // Send Signed VC over MQTT with flag proof
            let response = res.into_inner();
            let payload = serialize_msg(&enc::Did {
                did: response.did,
                challenge: response.challenge,
                vc: response.verifiable_credential,
                proof: false,
            });
            info!("Send Signed VC over MQTT");
            helper_send_mqtt(mqtt_client, payload, TOPIC_DID, channel_id).await?;
        }
        Err(e) => return Err(format!("Unable to Sign VC: {}", e)),
    };
    Ok(())
}

fn update_identity(
    db_client: &diesel::SqliteConnection,
    did: &str,
    is_verified: bool,
) -> Result<u32, String> {
    match db::update_identity(db_client, did, is_verified) {
        Ok(_) => {
            info!("Updated Indentity to Verified with DID: {}", did);
            return Ok(0);
        }
        Err(_) => return Err(format!("Unable to Update Identity with DID: {}", did)),
    };
}

fn update_identity_unverifiable(
    db_client: &diesel::SqliteConnection,
    did: &str,
) -> Result<u32, String> {
    match db::update_identity_to_unverifiable(db_client, did, true) {
        Ok(_) => {
            info!("Identity Marked as Unverifiable with DID: {}", did);
            return Ok(0);
        }
        Err(_) => {
            return Err(format!(
                "Unable to Make Identity Unverifiable with DID: {}",
                did
            ))
        }
    };
}

async fn receive_messages(
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
) -> Result<MqttMsgsReply, String> {
    let response = match mqtt_client
        .receive_mqtt_message(tonic::Request::new(MqttRequest::default()))
        .await
    {
        Ok(res) => res.into_inner(),
        Err(_) => return Err(format!("Unable to Receive MQTT Messages")),
    };
    Ok(response)
}

fn get_identity(db_client: &diesel::SqliteConnection, did: &str) -> Result<Identity, String> {
    match db::select_identity(&db_client, did) {
        Ok(res) => {
            info!("Message DID Found in DB, DID: {}", did);
            return Ok(res);
        }
        Err(_) => {
            make_identity(&db_client, did)?;
            match db::select_identity(&db_client, did) {
                Ok(res) => return Ok(res),
                Err(_) => return Err(format!("Unable to Create Identity Entry for DID: {}", did)),
            };
        }
    };
}

fn make_identity(db_client: &diesel::SqliteConnection, did: &str) -> Result<(), String> {
    match db::create_identity(&db_client, did, false) {
        Ok(_) => {
            info!("Created Identity Entry for DID: {}", did);
            return Ok(());
        }
        Err(_) => {
            return Err(format!("Unable to Create Identity Entry for DID: {}", did));
        }
    };
}
