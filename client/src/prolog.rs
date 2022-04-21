use public_ip;
use serde_json::json;
use std::env;

use crate::config::{
    ENV_ANNLINK_PUBLIC, ENV_DEVICE_ID, ENV_DEVICE_NAME, ENV_DEVICE_TYPE, ENV_THING_KEY,
    IDENTITY_SOCKET, MQTT_SOCKET, PUBLIC_CHANNEL_ID, STREAMS_SOCKET, TOPIC_IDENTITY,
};
use crate::db_module as db;
use crate::grpc_identity::iota_identifier_client::IotaIdentifierClient;
use crate::grpc_identity::{IotaIdentityCreationRequest, IotaIdentityRequest};
use crate::grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use crate::grpc_streams::iota_streamer_client::IotaStreamerClient;
use crate::grpc_streams::IotaStreamsRequest;
use crate::models::Identification;
use crate::models::{Channel, Thing};
use crate::mqtt_encoder as enc;
use crate::util::{generate_random_sequence, get_channel_ids, send_mqtt_message, serialize_msg};

pub async fn init() -> Result<bool, bool> {
    // ToDo: exclude when basic client ?
    let author_id = env::var(ENV_DEVICE_ID).expect("ENV for Author ID not Found");
    info!("ENV: {} = {}", ENV_DEVICE_ID, &author_id);
    let channel_ids = get_channel_ids();
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    let device_name = env::var(ENV_DEVICE_NAME).expect("ENV for Device Name not Found");
    info!("ENV: {} = {}", ENV_DEVICE_NAME, &device_name);
    let device_type = env::var(ENV_DEVICE_TYPE).expect("ENV for Device Type not Found");
    info!("ENV: {} = {}", ENV_DEVICE_TYPE, &device_type);
    // Connect to Database
    let db_client = db::establish_connection();
    // Connect to IOTA Streams Service
    let mut stream_client = connect_streams().await?;
    info!("Connected to Streams Service");
    // Connect to MQTT Service
    let mut mqtt_client = connect_mqtt().await?;
    info!("Connected to MQTT Service");
    // Connect to IOTA Streams Service
    //let mut stream_client = connect_streams().await?;
    info!("Connected to Streams Service");
    // Connect to Identity Service
    let mut identity_client = connect_identity().await?;
    info!("Connected to Identity Service");
    // Create Thing Entry
    match db::create_thing(&db_client, &thing_key) {
        Ok(_) => info!("New Thing Entry Created for Key: {}", &thing_key),
        Err(_) => error!("Thing Entry Not Created for Key: {}", &thing_key),
    };
    // Get Thing ID
    let thing = get_thing(&db_client, &thing_key)?;
    for channel in channel_ids {
        // Create Channel Entry
        match db::create_channel(&db_client, thing.id, &channel) {
            Ok(_) => info!("New Channel Entry Created for Key: {}", &channel),
            Err(_) => error!("Channel Entry Not Created for Key: {}", &channel),
        };
    }
    // Identity
    // Get own DID
    info!("Generate and Make Gateway DID Known");
    let _ = generate_gateway_did(
        &db_client,
        &mut identity_client,
        &mut mqtt_client,
        thing.id,
        &author_id,
        &device_name,
        &device_type,
    )
    .await?;
    // Create Config Entry
    info!("Make External IP Known");
    let ip = get_external_ip().await?;
    update_ip_address(&db_client, &ip, thing.id)?;
    // Create Public Streams Channel
    // Get Channel ID
    let channel = get_channel(&db_client, PUBLIC_CHANNEL_ID)?;
    // ToDo: exclude when basic client
    init_streams(&db_client, &mut stream_client, channel.id, &author_id).await?;
    info!("Gateway Successful Initialized");

    Ok(true)
}

async fn connect_mqtt() -> Result<MqttOperatorClient<tonic::transport::Channel>, bool> {
    let mqtt_client = match MqttOperatorClient::connect(format!("http://{}", MQTT_SOCKET)).await {
        Ok(res) => res,
        Err(e) => {
            error!("Error Connecting to MQTT-Service: {}", e);
            return Err(false);
        }
    };
    Ok(mqtt_client)
}

async fn connect_streams() -> Result<IotaStreamerClient<tonic::transport::Channel>, bool> {
    let stream_client =
        match IotaStreamerClient::connect(format!("http://{}", STREAMS_SOCKET)).await {
            Ok(res) => res,
            Err(e) => {
                error!("Error Connecting to Streams-Service: {}", e);
                return Err(false);
            }
        };
    Ok(stream_client)
}

async fn connect_identity() -> Result<IotaIdentifierClient<tonic::transport::Channel>, bool> {
    let identity_client =
        match IotaIdentifierClient::connect(format!("http://{}", IDENTITY_SOCKET)).await {
            Ok(res) => res,
            Err(e) => {
                println!("Error Connecting to Identity-Service: {}", e);
                return Err(false);
            }
        };
    Ok(identity_client)
}

fn get_thing(db_client: &diesel::SqliteConnection, thing_key: &str) -> Result<Thing, bool> {
    match db::select_thing(db_client, thing_key) {
        Ok(res) => {
            info!("Thing Entry Selected");
            return Ok(res);
        }
        Err(_) => {
            error!("Thing Entry Not Found with Key: {}", &thing_key);
            return Err(false);
        }
    };
}

pub async fn get_external_ip() -> Result<String, bool> {
    match public_ip::addr().await {
        Some(res) => {
            info!("Public IP Address: {:?}", res);
            return Ok(res.to_string());
        }
        None => {
            error!("Error Getting Public IP");
            return Err(false);
        }
    };
}

pub fn update_ip_address(
    db_client: &diesel::SqliteConnection,
    ip: &str,
    thing_id: i32,
) -> Result<(), bool> {
    match db::create_configuration(db_client, ip, 0) {
        Ok(_) => info!("Config Entry Created"),
        Err(_) => {
            error!("Config Entry Not Created");
            match db::update_configuration(&db_client, thing_id, "ip", &ip, 0) {
                Ok(_) => info!("Config Entry: IP Address Updated: {}", &ip),
                Err(_) => {
                    error!("Unable to Update IP Address");
                    return Err(false);
                }
            }
        }
    };
    Ok(())
}

pub async fn generate_gateway_did(
    db_client: &diesel::SqliteConnection,
    identity_client: &mut IotaIdentifierClient<tonic::transport::Channel>,
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
    thing_id: i32,
    author_id: &str,
    device_name: &str,
    device_type: &str,
) -> Result<Identification, bool> {
    // Check if DID already exists
    match db::select_identification(&db_client, thing_id) {
        Ok(res) => {
            info!("Identity Entry with Thing ID {} selected", thing_id);
            return Ok(res);
        }
        Err(_) => {
            // if not
            // Create new Identity
            error!(
                "Error Retrieving Thing Identification for Thing ID: {}",
                thing_id
            );
            info!("Creating New Identity with Identity Service");
            let vc = build_verifiable_credential(author_id, device_name, device_type);
            let identity = match identity_client
                .create_identity(IotaIdentityCreationRequest {
                    verifiable_credential: vc,
                })
                .await
            {
                Ok(res) => {
                    info!("Identity Entry for Thing ID {} Created", thing_id);
                    res.into_inner()
                }
                Err(_) => {
                    error!("Error Creating Identity for Thing ID: {}", thing_id);
                    return Err(false);
                }
            };
            // Save DID to DB
            match db::create_identification(
                &db_client,
                thing_id,
                &identity.did,
                &identity.verifiable_credential,
            ) {
                Ok(_) => {
                    info!("Identity Entry Created for DID: {}", &identity.did);
                }
                Err(_) => {
                    error!("Error Creating Identity Entry for DID: {}", &identity.did);
                    return Err(false);
                }
            };
            // Proof with Challenge and send to MQTT Identity Topic
            let response = match identity_client
                .proof_identity(tonic::Request::new(IotaIdentityRequest {
                    did: identity.did.clone(),
                    challenge: generate_random_sequence(),
                    verifiable_credential: identity.verifiable_credential.clone(),
                }))
                .await
            {
                Ok(res) => res.into_inner(),
                Err(e) => {
                    return {
                        error!("Unable to Verify Identity: {}", e);
                        Err(false)
                    }
                }
            };
            let payload = serialize_msg(&enc::Did {
                did: response.did,
                challenge: response.challenge,
                vc: response.verifiable_credential,
                proof: false,
            });
            let channel_keys = get_channel_ids();
            for channel in channel_keys {
                helper_send_mqtt(mqtt_client, payload.clone(), TOPIC_IDENTITY, &channel).await?;
            }
            return Ok(Identification {
                id: 0,
                thing_id: thing_id,
                did: identity.did,
                vc: Some(identity.verifiable_credential),
            });
        }
    };
}

fn build_verifiable_credential(id: &str, name: &str, device_type: &str) -> String {
    json!({
        "device": {
            "type": device_type,
            "id": id,
            "name": name
        }
    })
    .to_string()
}

async fn helper_send_mqtt(
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
    payload: Vec<u8>,
    topic: &str,
    channel_id: &str,
) -> Result<(), bool> {
    match send_mqtt_message(mqtt_client, payload, topic, channel_id).await {
        Ok(_) => info!("MQTT Message Transmitted to Service for Topic {}", topic),
        Err(e) => {
            error!("Error Sending MQTT Message: {}", e);
            return Err(false);
        }
    };
    Ok(())
}

fn get_channel(db_client: &diesel::SqliteConnection, channel_key: &str) -> Result<Channel, bool> {
    match db::select_channel(db_client, channel_key) {
        Ok(res) => {
            info!("Channel Entry Selected");
            return Ok(res);
        }
        Err(_) => {
            error!("Unable to Select Channel with Key: {}", channel_key);
            return Err(false);
        }
    };
}

// pub fn update_sensor_entries(
//     db_client: &diesel::SqliteConnection,
//     channel_id: i32,
//     sensor_list: Vec<Sensor>,
// ) -> Result<(), bool> {
//     for sensor in sensor_list {
//         match db::create_sensor_type(&db_client, &sensor.type_descr, &sensor.unit) {
//             Ok(_) => info!(
//                 "Sensor Type Entry Created for Sensor: {}",
//                 &sensor.type_descr
//             ),
//             Err(_) => error!(
//                 "Error Creating Sensor Type Entry for : {}",
//                 &sensor.type_descr
//             ),
//         };
//         let sensor_type = match db::select_sensor_type_by_desc(&db_client, &sensor.type_descr) {
//             Ok(res) => {
//                 info!("Sensor Type Entry Selected");
//                 res
//             }
//             Err(_) => {
//                 error!("Unable to Select Sensor Type Entry: {}", &sensor.type_descr);
//                 return Err(false);
//             }
//         };
//         match db::create_sensor(
//             &db_client,
//             db::SensorEntry {
//                 channel_id: channel_id,
//                 sensor_types_id: sensor_type.id,
//                 sensor_id: sensor.sensor_id.clone(),
//                 sensor_name: sensor.sensor_name.clone(),
//             },
//         ) {
//             Ok(_) => info!(
//                 "Sensor Entry Created for ID: {}, Name: {}",
//                 &sensor.sensor_id, &sensor.sensor_name
//             ),
//             Err(_) => error!(
//                 "Error Creating Sensor Entry for ID: {}, Name: {}",
//                 &sensor.sensor_id, &sensor.sensor_name
//             ),
//         };
//     }
//     Ok(())
// }

pub async fn init_streams(
    db_client: &diesel::SqliteConnection,
    stream_client: &mut IotaStreamerClient<tonic::transport::Channel>,
    channel_id: i32,
    author_id: &str,
) -> Result<(), bool> {
    match db::select_stream(&db_client, channel_id) {
        Ok(res) => info!(
            "Stream Entry Selected for Channel ID {}:{:?}",
            channel_id, res
        ),
        Err(_) => {
            // Create new Channel
            error!(
                "Error Selecting Stream Entry for Channel ID: {}",
                channel_id
            );
            info!("Create New Channel with Streams Service");
            let author = match stream_client
                .create_new_author(IotaStreamsRequest {
                    id: author_id.to_string(),
                    msg_type: 1, // CreateNewAuthor
                    link: "".to_string(),
                })
                .await
            {
                Ok(res) => res.into_inner(),
                Err(_) => {
                    println!("Error Creating New Channel for Author ID: {}", author_id);
                    return Err(false);
                }
            };
            env::set_var(ENV_ANNLINK_PUBLIC, &author.link);
            info!("Announcement Link: {}", &author.link);
            info!("Saved in ENV: {}", ENV_ANNLINK_PUBLIC);
            match db::create_stream(
                &db_client,
                db::StreamsEntry {
                    channel_id: channel_id,
                    ann_link: author.link.clone(),
                    sub_link: "".to_string(),
                    key_link: "".to_string(),
                    msg_link: "".to_string(),
                },
            ) {
                Ok(_) => info!("Streams Entry Created for Channel ID: {}", channel_id),
                Err(_) => error!(
                    "Error Creating Streams Entry for Channel ID: {}",
                    channel_id
                ),
            }
        }
    };
    Ok(())
}
