use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::env;
use std::fmt::Debug;
use std::str::FromStr;

use crate::config::{
    ENV_CHANNELS_KEY, ENV_IS_FACTORY, ENV_SENSOR_KEYS, ENV_THING_KEY, ENV_THING_PWD,
    IDENTITY_SOCKET, MQTT_SOCKET, STREAMS_SOCKET, TOPIC_STREAM,
};
use crate::db_module as db;
use crate::grpc_identity::iota_identifier_client::IotaIdentifierClient;
use crate::grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use crate::grpc_mqtt::MqttRequest;
use crate::grpc_streams::iota_streamer_client::IotaStreamerClient;
use crate::models::{Channel, Identification, Thing};
use crate::mqtt_encoder as enc;

pub fn serialize_msg<T: prost::Message>(msg: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.reserve(msg.encoded_len());
    msg.encode(&mut buf).unwrap();
    buf
}

pub fn is_factory() -> bool {
    let is_factory_str = env::var(ENV_IS_FACTORY).expect("ENV for Client Switch not Found");
    let is_factory: bool = match is_factory_str.as_str() {
        "true" => true,
        "t" => true,
        "false" => false,
        "f" => false,
        _ => false,
    };
    is_factory
}
/// Get Channel IDs, split string over seperator ';'
pub fn get_channel_ids() -> Vec<String> {
    let channel_ids = env::var(ENV_CHANNELS_KEY).expect("ENV for Channel Keys not Found");
    info!("ENV: {} = {}", ENV_CHANNELS_KEY, &channel_ids);
    let keys_vec: Vec<_> = channel_ids.split(';').collect();
    let keys_string: Vec<String> = keys_vec.iter().map(|s| s.to_string()).collect();
    keys_string
}

/// Get Sensor IDs, split string over seperator ';'
pub fn get_sensor_ids() -> Vec<String> {
    let sensor_ids = env::var(ENV_SENSOR_KEYS).expect("ENV for Sensor Keys not Found");
    info!("ENV: {} = {}", ENV_SENSOR_KEYS, &sensor_ids);
    let keys_vec: Vec<_> = sensor_ids.split(';').collect();
    let keys_string: Vec<String> = keys_vec.iter().map(|s| s.to_string()).collect();
    keys_string
}

pub fn update_streams_entry(
    db_client: &diesel::SqliteConnection,
    link: &str,
    num_subs: i32,
    query: &str,
    channel_id: i32,
) -> Result<(), String> {
    match db::update_stream(db_client, channel_id, query, link, num_subs) {
        Ok(_) => info!("Update {} to {}", query, link),
        Err(_) => return Err(format!("Unable to Update {} Entry to {}", query, link)),
    };
    Ok(())
}

pub async fn send_mqtt_message(
    client: &mut MqttOperatorClient<tonic::transport::Channel>,
    payload: Vec<u8>,
    topic: &str,
    channel_id: &str,
) -> Result<String, String> {
    let _response = match client
        .send_mqtt_message(tonic::Request::new(MqttRequest {
            id: env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found"),
            postfix: "pub".to_string(),
            pwd: env::var(ENV_THING_PWD).expect("ENV for Thing PWD not Found"),
            channel: channel_id.to_string(),
            topic: topic.to_string(),
            message: payload,
        }))
        .await
    {
        Ok(res) => return Ok(res.into_inner().status),
        Err(e) => return Err(format!("Error:{}", e)),
    };
}

pub async fn send_sublink(
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
    db_client: &diesel::SqliteConnection,
    sub_link: &str,
    channel_id: &str,
) -> Result<String, String> {
    let sub_link = sub_link.to_string();
    if sub_link.is_empty() {
        return Ok("No Subscription Link Found".to_string());
    }
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    let thing = get_thing(db_client, &thing_key)?;
    let identity = get_identification(&db_client, thing.id)?;
    let payload = serialize_msg(&enc::Streams {
        did: identity.did,
        announcement_link: "".to_string(),
        subscription_link: sub_link,
        keyload_link: "".to_string(),
        vc: match identity.vc {
            Some(r) => r,
            None => "".to_string(),
        },
    });
    helper_send_mqtt(mqtt_client, payload, TOPIC_STREAM, channel_id).await?;
    Ok("Send Subscription Link".to_string())
}

pub fn generate_random_sequence() -> String {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    rand_string
}

pub fn parse_env<T>(env_var_name: &str) -> T
where
    T: FromStr,
    T::Err: Debug,
{
    let var = env::var(env_var_name).unwrap();
    var.parse().unwrap()
}

pub async fn connect_mqtt() -> Result<MqttOperatorClient<tonic::transport::Channel>, String> {
    let mqtt_client = match MqttOperatorClient::connect(format!("http://{}", MQTT_SOCKET)).await {
        Ok(res) => res,
        Err(e) => {
            return Err(format!("Error Connecting to MQTT-Service: {}", e));
        }
    };
    Ok(mqtt_client)
}

pub async fn connect_streams() -> Result<IotaStreamerClient<tonic::transport::Channel>, String> {
    let stream_client =
        match IotaStreamerClient::connect(format!("http://{}", STREAMS_SOCKET)).await {
            Ok(res) => res,
            Err(e) => {
                return Err(format!("Error Connecting to Streams-Service: {}", e));
            }
        };
    Ok(stream_client)
}

pub async fn connect_identity() -> Result<IotaIdentifierClient<tonic::transport::Channel>, String> {
    let identity_client =
        match IotaIdentifierClient::connect(format!("http://{}", IDENTITY_SOCKET)).await {
            Ok(res) => res,
            Err(e) => {
                return Err(format!("Error Connecting to Identity-Service: {}", e));
            }
        };
    Ok(identity_client)
}

pub async fn helper_send_mqtt(
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
    payload: Vec<u8>,
    topic: &str,
    channel_id: &str,
) -> Result<(), String> {
    match send_mqtt_message(mqtt_client, payload, topic, channel_id).await {
        Ok(_) => info!("MQTT Message Transmitted to Service for Topic {}", topic),
        Err(e) => {
            error!("Error Sending MQTT Message: {}", e);
            return Err(format!(
                "Unable to Transmitted MQTT Messagefor Topic {}",
                topic
            ));
        }
    };
    Ok(())
}

pub fn get_channel(
    db_client: &diesel::SqliteConnection,
    channel_key: &str,
) -> Result<Channel, String> {
    match db::select_channel(db_client, channel_key) {
        Ok(res) => {
            info!("Channel Entry Selected");
            return Ok(res);
        }
        Err(_) => {
            error!("Unable to Select Channel with Key: {}", channel_key);
            return Err(format!(
                "Unable to Select Channel with Key: {}",
                channel_key
            ));
        }
    };
}

pub fn get_thing(db_client: &diesel::SqliteConnection, thing_key: &str) -> Result<Thing, String> {
    match db::select_thing(db_client, thing_key) {
        Ok(res) => {
            info!("Thing Entry Selected with Key: {}", &thing_key);
            return Ok(res);
        }
        Err(_) => {
            error!("Thing Entry Not Found with Key: {}", &thing_key);
            return Err(format!("Unable to Select Thing with Key: {}", &thing_key));
        }
    };
}

pub fn get_identification(
    db_client: &diesel::SqliteConnection,
    thing_id: i32,
) -> Result<Identification, String> {
    match db::select_identification(&db_client, thing_id) {
        Ok(res) => {
            info!("Selected Identity for Thing ID: {}", thing_id);
            return Ok(res);
        }
        Err(_) => {
            return Err(format!(
                "Unable to Select Identity for Thing ID: {}",
                thing_id
            ))
        }
    };
}
