#![allow(dead_code)]
use prost::Message;

use std::env;
use std::io::Cursor;

use crate::config::{
    ENV_CHANNEL_KEY, ENV_DEVICE_ID, ENV_THING_KEY, ENV_TOTAL_NUM_SUBSCRIBER, TOPIC_COMMAND,
    TOPIC_DID, TOPIC_IDENTITY, TOPIC_SETTING, TOPIC_STREAM, ENV_THING_PWD,
};
use crate::db_module as db;
use crate::grpc_identity::iota_identifier_client::IotaIdentifierClient;
use crate::grpc_identity::IotaIdentityRequest;
use crate::grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use crate::grpc_mqtt::{MqttMsgsReply, MqttRequest};
use crate::grpc_streams::iota_streamer_client::IotaStreamerClient;
use crate::grpc_streams::IotaStreamsRequest;
use crate::models::Identity;
use crate::mqtt_encoder as enc;
use crate::util::{
    connect_identity, connect_mqtt, connect_streams, get_channel, get_identification, get_thing,
    helper_send_mqtt, parse_env, serialize_msg, update_streams_entry,
};
use std::fs;
use std::path::Path;

pub async fn receive_mqtt_messages() -> Result<String, String> {
    info!("--- receive_mqtt_messages() ---");
    let mut mqtt_client = connect_mqtt().await?;
    info!("Receive MQTT Messages ...");
    let mut response = receive_messages(&mut mqtt_client).await?;
    for (payload, topic) in response.messages.iter_mut().zip(response.topics) {
        let result = match topic.as_str() {
            TOPIC_DID => mqtt_identity(payload.to_vec()).await,
            TOPIC_STREAM => mqtt_streams(payload.to_vec()).await,
            TOPIC_SETTING => mqtt_settings(payload.to_vec()).await,
            TOPIC_COMMAND => mqtt_command(payload.to_vec()).await,
            TOPIC_IDENTITY => mqtt_first_verification(payload.to_vec()).await,
            e => Err(format!("Topic {} not Found", e)),
            // Ignore Topics identity & sensors
        };
        match result {
            Ok(_) => (),
            Err(e) => error!("{}", e),
        }
    }
    Ok("Exit with Success: receive_mqtt_messages()".to_string())
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

pub async fn mqtt_streams(payload: Vec<u8>) -> Result<u32, String> {
    info!("--- mqtt_streams() ---");
    let author_id = env::var(ENV_DEVICE_ID).expect("ENV for Author ID not Found");
    info!("ENV: {} = {}", ENV_DEVICE_ID, &author_id);
    let channel_key = env::var(ENV_CHANNEL_KEY).expect("ENV for Channel Key not Found");
    info!("ENV: {} = {}", ENV_CHANNEL_KEY, &channel_key);
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    // Decode Payload
    let msg = match enc::Streams::decode(&mut Cursor::new(payload)) {
        Ok(res) => res,
        Err(e) => return Err(format!("Unable to Decode Payload: {}", e)),
    };
    // Connect to MQTT Service
    let mut mqtt_client = connect_mqtt().await?;
    // Connect to IOTA Streams Service
    let mut stream_client = connect_streams().await?;
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
    if !msg.subscription_link.is_empty() && is_verified {
        add_subscriber(
            &db_client,
            &mut stream_client,
            &mut mqtt_client,
            &msg.subscription_link,
            &author_id,
            &channel_key,
            &thing_key,
        )
        .await?;
    }
    info!("Streams Message Processed");
    Ok(0)
}

pub async fn mqtt_identity(payload: Vec<u8>) -> Result<u32, String> {
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
        proof_identity(&mut identity_client, &mut mqtt_client, msg).await?;
    // Thing should verify received DID
    } else if !msg.proof && !is_thing {
        info!("Verify Participant's Identity");
        verify_identity(&mut identity_client, &db_client, msg).await?;
    }
    Ok(0)
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

async fn add_subscriber(
    db_client: &diesel::SqliteConnection,
    stream_client: &mut IotaStreamerClient<tonic::transport::Channel>,
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
    sub_link: &str,
    author_id: &str,
    channel_key: &str,
    thing_key: &str,
) -> Result<u32, String> {
    info!("ENV: {} = {}", ENV_CHANNEL_KEY, &channel_key);
    match stream_client
        .add_subscriber(tonic::Request::new(IotaStreamsRequest {
            id: author_id.to_string(),
            link: sub_link.to_string(),
            msg_type: 2, //  CreateNewSubscriber
        }))
        .await
    {
        Ok(res) => res.into_inner(),
        Err(e) => return Err(format!("Error Adding Subscriber: {}", e)),
    };
    // Get number of subscribers
    let channel = get_channel(&db_client, channel_key)?;
    let num_subscribers = match db::select_stream(&db_client, channel.id) {
        Ok(r) => match r.num_subs {
            Some(r) => r + 1,
            None => return Err("Unable to Get Number of Subscribers".to_string()),
        },
        Err(e) => return Err(format!("Unable to Select Streams Entry: {}", e)),
    };
    update_streams_entry(&db_client, "", num_subscribers, "num_subs", channel.id)?;
    let total_num_subscribers = parse_env::<i32>(ENV_TOTAL_NUM_SUBSCRIBER);
    if num_subscribers != total_num_subscribers {
        info!("Number of Subscribers: {}", num_subscribers);
        return Ok(0);
    }
    // Get Channel ID
    let channel = get_channel(&db_client, &channel_key)?;
    // Get Thing ID
    let thing = get_thing(&db_client, &thing_key)?;
    // Get own DID
    let identity = get_identification(&db_client, thing.id)?;
    // Generate Keyload
    let response = match stream_client
        .create_keyload(tonic::Request::new(IotaStreamsRequest {
            id: author_id.to_string(),
            link: "".to_string(),
            msg_type: 7, //  CreateKeyload
        }))
        .await
    {
        Ok(res) => res.into_inner(),
        Err(e) => return Err(format!("Unable to Create Keyload Link: {}", e)),
    };
    // Send Keyload over MQTT
    let payload = serialize_msg(&enc::Streams {
        announcement_link: "".to_string(),
        subscription_link: "".to_string(),
        keyload_link: response.link.clone(),
        did: identity.did,
        vc: match identity.vc {
            Some(r) => r,
            None => "".to_string(),
        },
    });
    info!("Send Keyload over MQTT");
    helper_send_mqtt(mqtt_client, payload, TOPIC_STREAM).await?;
    // Save Keyload Link
    update_streams_entry(&db_client, &response.link, 0, "keyload", channel.id)?;
    Ok(0)
}

async fn proof_identity(
    identity_client: &mut IotaIdentifierClient<tonic::transport::Channel>,
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
    identity: enc::Did,
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
            helper_send_mqtt(mqtt_client, payload, TOPIC_DID).await?;
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
        .receive_mqtt_message(tonic::Request::new(
            MqttRequest {
                id: env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found"),
                pwd: env::var(ENV_THING_PWD).expect("ENV for Thing PWD not Found"),
                channel: env::var(ENV_CHANNEL_KEY).expect("ENV for Channel Key not Found"),
                topic: "".to_string(),
                message: vec![],
        }))
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
