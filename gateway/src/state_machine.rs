#![allow(dead_code)]
use prost::Message;
use public_ip;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sensor_grpc_adapter as adapter;
use serde_json::json;
use std::env;
use std::io::Cursor;
use tokio::sync::mpsc;

use crate::config::{
    load_config_file, ENV_CHANNEL_KEY, ENV_DEVICE_ID, ENV_DEVICE_NAME, ENV_DEVICE_TYPE,
    ENV_THING_KEY, IDENTITY_SOCKET, MQTT_SOCKET, STREAMS_SOCKET,
};
use crate::db_module as db;
use crate::grpc_identity::iota_identifier_client::IotaIdentifierClient;
use crate::grpc_identity::{IotaIdentityCreationRequest, IotaIdentityRequest};
use crate::grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use crate::grpc_mqtt::MqttRequest;
use crate::grpc_streams::iota_streamer_client::IotaStreamerClient;
use crate::grpc_streams::{IotaStreamsRequest, IotaStreamsSendMessageRequest};
use crate::models::Identification;
use crate::mqtt_encoder as enc;
use std::fs;
use std::path::Path;
use tokio::time::{sleep, Duration};

const TOPIC_DID: &str = "did";
const TOPIC_SENSOR_VALUE: &str = "sensors";
const TOPIC_SETTING: &str = "settings";
const TOPIC_IDENTITY: &str = "identity";
const TOPIC_STREAM: &str = "stream";
const TOPIC_COMMAND: &str = "command";

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

pub async fn init() -> bool {
    let cfg = load_config_file();
    let author_id = env::var(ENV_DEVICE_ID).expect("ENV for Author ID not Found");
    info!("ENV: {} = {}", ENV_DEVICE_ID, &author_id);
    let channel_key = env::var(ENV_CHANNEL_KEY).expect("ENV for Channel Key not Found");
    info!("ENV: {} = {}", ENV_CHANNEL_KEY, &channel_key);
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    info!("ENV: {} = {}", ENV_THING_KEY, &thing_key);
    let device_name = env::var(ENV_DEVICE_NAME).expect("ENV for Device Name not Found");
    info!("ENV: {} = {}", ENV_DEVICE_NAME, &device_name);
    let device_type = env::var(ENV_DEVICE_TYPE).expect("ENV for Device Type not Found");
    info!("ENV: {} = {}", ENV_DEVICE_TYPE, &device_type);
    // Connect to Database
    let db_client = db::establish_connection();
    // Connect to MQTT Service
    let mut mqtt_client = match MqttOperatorClient::connect(format!("http://{}", MQTT_SOCKET)).await
    {
        Ok(res) => res,
        Err(e) => {
            error!("Error Connecting to MQTT-Service: {}", e);
            return false;
        }
    };
    info!("Connected to MQTT Service");
    // Connect to IOTA Streams Service
    let mut stream_client =
        match IotaStreamerClient::connect(format!("http://{}", STREAMS_SOCKET)).await {
            Ok(res) => res,
            Err(e) => {
                println!("Error Connecting to Streams Client: {}", e);
                return false;
            }
        };
    info!("Connected to Streams Service");
    // Connect to Identity Service
    let mut identity_client =
        match IotaIdentifierClient::connect(format!("http://{}", IDENTITY_SOCKET)).await {
            Ok(res) => res,
            Err(e) => {
                println!("Client Identity Service Error: {}", e);
                return false;
            }
        };
    info!("Connected to Identity Service");
    // Create Thing Entry
    match db::create_thing(&db_client, &thing_key) {
        Ok(_) => info!("New Thing Entry Created for Key: {}", &thing_key),
        Err(_) => error!("Thing Entry Not Created for Key: {}", &thing_key),
    };
    // Get Thing ID
    let thing = match db::select_thing(&db_client, &thing_key) {
        Ok(res) => {
            info!("Thing Entry Selected");
            res
        }
        Err(_) => {
            error!("Thing Entry Not Found with Key: {}", &thing_key);
            return false;
        }
    };
    // Create Channel Entry
    match db::create_channel(&db_client, thing.id, &channel_key) {
        Ok(_) => info!("New Channel Entry Created for Key: {}", &channel_key),
        Err(_) => error!("Channel Entry Not Created for Key: {}", &channel_key),
    };
    // Create Config Entry
    let ip = match public_ip::addr().await {
        Some(res) => {
            info!("Public IP Address: {:?}", res);
            res.to_string()
        }
        None => {
            error!("Error Getting Public IP");
            return false;
        }
    };
    match db::create_configuration(&db_client, &ip, 0) {
        Ok(_) => info!("Config Entry Created"),
        Err(_) => {
            error!("Config Entry Not Created");
            match db::update_configuration(&db_client, thing.id, "ip", &ip, 0) {
                Ok(_) => info!("Config Entry: IP Address Updated: {}", &ip),
                Err(_) => {
                    error!("Unable to Update IP Address");
                    return false;
                }
            }
        }
    };
    let payload = serialize_msg(&enc::Setting {
        ip: ip.clone(),
        pk_timestamp: 0,
        pk: "".to_string(),
    });
    match send_mqtt_message(&mut mqtt_client, payload, TOPIC_SETTING).await {
        Ok(_) => info!(
            "MQTT Message Transmitted to Service for Topic {}",
            TOPIC_SETTING
        ),
        Err(e) => {
            error!("Error Sending MQTT Message: {}", e);
            return false;
        }
    };
    // Get Channel ID
    let channel = match db::select_channel(&db_client, &channel_key) {
        Ok(res) => {
            info!("Channel Entry Selected");
            res
        }
        Err(_) => {
            error!("Unable to Select Channel with Key: {}", &channel_key);
            return false;
        }
    };
    // Create Entries: SensorType, Sensor
    for sensor in cfg.sensors.list {
        match db::create_sensor_type(&db_client, &sensor.type_descr, &sensor.unit) {
            Ok(_) => info!(
                "Sensor Type Entry Created for Sensor: {}",
                &sensor.type_descr
            ),
            Err(_) => error!(
                "Error Creating Sensor Type Entry for : {}",
                &sensor.type_descr
            ),
        };
        let sensor_type = match db::select_sensor_type_by_desc(&db_client, &sensor.type_descr) {
            Ok(res) => {
                info!("Sensor Type Entry Selected");
                res
            }
            Err(_) => {
                error!("Unable to Select Sensor Type Entry: {}", &sensor.type_descr);
                return false;
            }
        };
        match db::create_sensor(
            &db_client,
            db::SensorEntry {
                channel_id: channel.id,
                sensor_types_id: sensor_type.id,
                sensor_id: sensor.sensor_id.clone(),
                sensor_name: sensor.sensor_name.clone(),
            },
        ) {
            Ok(_) => info!(
                "Sensor Entry Created for ID: {}, Name: {}",
                &sensor.sensor_id, &sensor.sensor_name
            ),
            Err(_) => error!(
                "Error Creating Sensor Entry for ID: {}, Name: {}",
                &sensor.sensor_id, &sensor.sensor_name
            ),
        };
    }
    // Identity
    // Get own DID
    // ToDo: Send DID over MQTT
    let identity = match db::select_identification(&db_client, thing.id) {
        Ok(res) => {
            info!("Identity Entry with Thing ID {} selected", thing.id);
            res
        }
        Err(_) => {
            // Create new Identity
            error!(
                "Error Retrieving Thing Identification for Thing ID: {}",
                thing.id
            );
            info!("Creating New Identity with Identity Service");
            let identity = match identity_client
                .create_identity(IotaIdentityCreationRequest {
                    device_id: author_id.clone(),
                    device_name: device_name,
                    device_type: device_type,
                })
                .await
            {
                Ok(res) => {
                    info!("Identity Entry for Thing ID {} Created", thing.id);
                    res.into_inner()
                }
                Err(_) => {
                    error!("Error Creating Identity for Thing ID: {}", thing.id);
                    return false;
                }
            };
            match db::create_identification(
                &db_client,
                thing.id,
                &identity.did,
                &identity.verifiable_credential,
            ) {
                Ok(_) => {
                    info!("Identity Entry Created for DID: {}", &identity.did);
                }
                Err(_) => {
                    error!("Error Creating Identity Entry for DID: {}", &identity.did);
                    return false;
                }
            };
            Identification {
                id: 0,
                thing_id: thing.id,
                did: identity.did,
                vc: Some(identity.verifiable_credential),
            }
        }
    };
    // On Start-Up Check if Entries have been made
    // Initialize Streams Connection
    match db::select_stream(&db_client, channel.id) {
        Ok(res) => info!(
            "Stream Entry Selected for Channel ID {}:{:?}",
            channel.id, res
        ),
        Err(_) => {
            // Create new Channel
            error!(
                "Error Selecting Stream Entry for Channel ID: {}",
                channel.id
            );
            info!("Create New Channel with Streams Service");
            let author = match stream_client
                .create_new_author(IotaStreamsRequest {
                    id: author_id.clone(),
                    msg_type: 1, // CreateNewAuthor
                    link: "".to_string(),
                })
                .await
            {
                Ok(res) => res.into_inner(),
                Err(_) => {
                    println!("Error Creating New Channel for Author ID: {}", author_id);
                    return false;
                }
            };
            info!("Announcement Link: {}", &author.link);
            match db::create_stream(
                &db_client,
                db::StreamsEntry {
                    channel_id: channel.id,
                    ann_link: author.link.clone(),
                    sub_link: "".to_string(),
                    key_link: "".to_string(),
                    msg_link: "".to_string(),
                },
            ) {
                Ok(_) => info!("Streams Entry Created for Channel ID: {}", channel.id),
                Err(_) => error!(
                    "Error Creating Streams Entry for Channel ID: {}",
                    channel.id
                ),
            }
            let payload = serialize_msg(&enc::Streams {
                announcement_link: author.link,
                subscription_link: "".to_string(),
                keyload_link: "".to_string(),
                did: identity.did,
                vc: match identity.vc {
                    Some(r) => r,
                    None => "".to_string(),
                },
            });
            info!("Sending MQTT Message with Announcement Link to Streams Topic");
            match send_mqtt_message(&mut mqtt_client, payload, TOPIC_STREAM).await {
                Ok(_) => info!("Message Transmitted to MQTT Service"),
                Err(_) => {
                    error!("Error Sending MQTT Message with Announcement Link");
                    return false;
                }
            };
        }
    };
    info!("Gateway Successful Initialized");
    true
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

pub async fn receive_mqtt_messages() -> Result<String, String> {
    info!("--- receive_mqtt_messages() ---");
    let mut mqtt_client = match MqttOperatorClient::connect(format!("http://{}", MQTT_SOCKET)).await
    {
        Ok(res) => {
            info!("Connected to MQTT Service");
            res
        }
        Err(e) => return Err(format!("Error Connecting to MQTT-Service: {}", e)),
    };
    info!("Receive MQTT Messages ...");
    let mut response = match mqtt_client
        .receive_mqtt_message(tonic::Request::new(MqttRequest::default()))
        .await
    {
        Ok(res) => res.into_inner(),
        Err(_) => return Err(format!("Unable to Receive MQTT Messages")),
    };

    for (payload, topic) in response.messages.iter_mut().zip(response.topics) {
        let _result = match topic.as_str() {
            TOPIC_DID => mqtt_identity(payload.to_vec(), &mut mqtt_client).await,
            TOPIC_STREAM => mqtt_streams(payload.to_vec()).await,
            TOPIC_SETTING => mqtt_settings(payload.to_vec()).await,
            TOPIC_COMMAND => mqtt_command(payload.to_vec()).await,
            e => {
                error!("Topic {} not Found", e);
                Err(format!("Topic {} not Found", e.to_string()))
            } // Ignore Topics identity & sensors
        };
    }
    Ok("Exit with Success: receive_mqtt_messages()".to_string())
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
            Err(e) => return Err(format!("Error Connecting to Streams Client: {}", e)),
        };
    // Connect to Database
    let db_client = db::establish_connection();
    // Verify that Message wasn't sent from this Thing
    // Get Thing ID
    let thing = match db::select_thing(&db_client, &thing_key) {
        Ok(res) => {
            info!("Selected Thing with Key: {}", &thing_key);
            res
        }
        Err(e) => return Err(format!("Unable to Select Thing with Key: {}", e)),
    };
    // Get own DID
    let identity = match db::select_identification(&db_client, thing.id) {
        Ok(res) => {
            info!("Selected Identity for Thing ID: {}", thing.id);
            res
        }
        Err(_) => {
            return Err(format!(
                "Unable to Select Identity for Thing ID: {}",
                thing.id
            ))
        }
    };
    if identity.did.eq(&msg.did) {
        return Err(format!("Received Own Message, DID: {}", &msg.did));
    }
    // Check if Participant is Verified (DID)
    // Query Saved DIDs: If DID not verified send request to Proof DID
    // If DID not in DB, create new Entry and request verification
    let identity_verified = match db::select_identity(&db_client, &msg.did) {
        Ok(res) => {
            info!("Message DID Found in DB, DID: {}", &msg.did);
            let verified = match res.verified {
                Some(r) => r,
                None => false,
            };
            if verified {
                info!("DID Already Verified");
                true
            } else {
                info!("DID Not Verified");
                // Send Identity Request over MQTT
                let payload = serialize_msg(&enc::Did {
                    did: msg.did.clone(),
                    challenge: generate_random_sequence(),
                    vc: msg.vc.clone(),
                    proof: true,
                });
                info!("Send MQTT Message with Verification Request");
                match send_mqtt_message(&mut mqtt_client, payload, TOPIC_DID).await {
                    Ok(_) => info!("Message Send Successfully"),
                    Err(_) => return Err("Unable to Send Message to Topic DID".to_string()),
                };
                // wait and Loop till verified
                info!("Wait for Verification Response");
                let mut max_iteration = 0;
                loop {
                    sleep(Duration::from_millis(10000)).await;
                    max_iteration += 1;
                    match db::select_identity(&db_client, &msg.did) {
                        Ok(res) => {
                            let verified = match res.verified {
                                Some(r) => r,
                                None => false,
                            };
                            if verified {
                                info!("Verification Successfull");
                                break;
                            }
                            if max_iteration > 10 {
                                error!("Unable to Verify DID: Timeout");
                                break;
                            }
                        }
                        Err(_) => {
                            return Err(format!("Unable to Select Identity for DID: {}", &msg.did))
                        }
                    }
                }
                true
            }
        }
        Err(_) => {
            match db::create_identity(&db_client, &msg.did, false) {
                Ok(_) => info!("Created Identity Entry for DID: {}", &msg.did),
                Err(_) => {
                    return Err(format!(
                        "Unable to Create Identity Entry for DID: {}",
                        &msg.did
                    ))
                }
            };
            // Send Identity Request over MQTT
            let payload = serialize_msg(&enc::Did {
                did: msg.did.clone(),
                challenge: generate_random_sequence(),
                vc: msg.vc.clone(),
                proof: true,
            });
            info!("Send MQTT Message with Verification Request");
            let _res = send_mqtt_message(&mut mqtt_client, payload, TOPIC_DID).await;
            // wait and Loop till verified
            info!("Wait for Verification Response");
            let mut max_iteration = 0;
            loop {
                sleep(Duration::from_millis(10000)).await;
                max_iteration += 1;
                match db::select_identity(&db_client, &msg.did) {
                    Ok(res) => {
                        let verified = match res.verified {
                            Some(r) => r,
                            None => false,
                        };
                        if verified {
                            info!("Verification Successfull");
                            break;
                        }
                        if max_iteration > 10 {
                            error!("Unable to Verify DID: Timeout");
                            break;
                        }
                    }
                    Err(_) => {
                        return Err(format!("Unable to Select Identity for DID: {}", &msg.did))
                    }
                }
            }
            true
        }
    };
    if !msg.subscription_link.is_empty() && identity_verified {
        info!("Subscription Link Found, Adding new Subsriber");
        match stream_client
            .add_subscriber(tonic::Request::new(IotaStreamsRequest {
                id: author_id,
                link: msg.subscription_link,
                msg_type: 2, //  CreateNewSubscriber
            }))
            .await
        {
            Ok(res) => {
                let response = res.into_inner();
                // Get Channel ID
                let channel = match db::select_channel(&db_client, &channel_key) {
                    Ok(res) => {
                        info!("Selected Channel with Key: {}", &channel_key);
                        res
                    }
                    Err(_) => {
                        return Err(format!(
                            "Unable to Select Channel with Key: {}",
                            &channel_key
                        ))
                    }
                };
                // Get Thing ID
                let thing = match db::select_thing(&db_client, &thing_key) {
                    Ok(res) => {
                        info!("Selected Thing with Key: {}", &thing_key);
                        res
                    }
                    Err(_) => {
                        return Err(format!("Unable to Select Thing with Key: {}", &thing_key))
                    }
                };
                // Get own DID
                let identity = match db::select_identification(&db_client, thing.id) {
                    Ok(res) => {
                        info!("Selected Own DID");
                        res
                    }
                    Err(_) => {
                        return Err(format!(
                            "Unable to Select Thing Identification for ID: {}",
                            thing.id
                        ))
                    }
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
                let _res = send_mqtt_message(&mut mqtt_client, payload, TOPIC_STREAM).await;
                // Save Keyload Link
                match db::update_stream(&db_client, channel.id, "keyload", &response.link) {
                    Ok(_) => return Ok(0),
                    Err(_) => return Err(format!("Unable to Update Streams Entry")),
                };
            }
            Err(e) => return Err(format!("Error Adding Subscriber: {}", e)),
        };
    }
    info!("Streams Message Processed");
    Ok(0)
}

pub async fn mqtt_command(payload: Vec<u8>) -> Result<u32, String> {
    // Not Implemented
    info!("--- mqtt_command() --- not Implemented");
    let _msg = enc::Command::decode(&mut Cursor::new(payload));
    Ok(0)
}

pub async fn mqtt_identity(
    payload: Vec<u8>,
    mqtt_client: &mut MqttOperatorClient<tonic::transport::Channel>,
) -> Result<u32, String> {
    info!("--- mqtt_identity() ---");
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
    // Connect to Identity Service
    let mut identity_client =
        match IotaIdentifierClient::connect(format!("http://{}", IDENTITY_SOCKET)).await {
            Ok(res) => res,
            Err(e) => return Err(format!("Client Identity Service Error: {}", e)),
        };
    // Connect to Database
    let db_client = db::establish_connection();
    // Decode Message
    let msg = match enc::Did::decode(&mut Cursor::new(payload)) {
        Ok(res) => res,
        Err(e) => return Err(format!("Decoding Error: {}", e)),
    };
    // Get DID
    let thing = match db::select_thing(&db_client, &thing_key) {
        Ok(res) => res,
        Err(e) => return Err(format!("Thing Not Found: {}", e)),
    };
    let identity = match db::select_identification(&db_client, thing.id) {
        Ok(res) => res,
        Err(e) => return Err(format!("Identification Not Found: {}", e)),
    };
    // Check if DID in Message is same as DID of this thing
    let is_thing = if identity.did.eq(&msg.did) {
        true
    } else {
        false
    };
    // If Thing is requested to proof identity and requested DID is same
    // Sign VC with challenge
    if msg.proof && is_thing {
        let _response = match identity_client
            .proof_identity(tonic::Request::new(IotaIdentityRequest {
                did: msg.did,
                challenge: msg.challenge,
                verifiable_credential: msg.vc,
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
                let _res = send_mqtt_message(mqtt_client, payload, TOPIC_DID).await;
            }
            Err(e) => return Err(format!("Decoding Error: {}", e)),
        };
    // Thing should verify received DID
    } else if !msg.proof && !is_thing {
        let _response = match identity_client
            .verify_identity(tonic::Request::new(IotaIdentityRequest {
                did: msg.did,
                challenge: msg.challenge,
                verifiable_credential: msg.vc,
            }))
            .await
        {
            Ok(res) => {
                let response = res.into_inner();
                // Check if Verification was a success, GRPC Call returns Status = "Verified"
                if response.status.eq("Verified") {
                    // Save Answer to DB
                    // Check if Entry Exists
                    match db::select_identity(&db_client, &response.did) {
                        Ok(response) => {
                            match db::update_identity(&db_client, &response.did, true) {
                                Ok(_) => return Ok(0),
                                Err(e) => return Err(format!("Error Updating Identity: {}", e)),
                            };
                        }
                        Err(_) => {
                            match db::create_identity(&db_client, &response.did, true) {
                                Ok(_) => return Ok(0),
                                Err(e) => {
                                    return Err(format!("Error Creating Verified Identity: {}", e))
                                }
                            };
                        }
                    };
                }
            }
            Err(e) => return Err(format!("Decoding Error: {}", e)),
        };
    }
    Ok(0)
}

pub async fn mqtt_settings(payload: Vec<u8>) -> Result<u32, String> {
    info!("--- mqtt_settings() ---");
    let thing_key = env::var(ENV_THING_KEY).expect("ENV for Thing Key not Found");
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

pub fn serialize_msg<T: prost::Message>(msg: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.reserve(msg.encoded_len());
    msg.encode(&mut buf).unwrap();
    buf
}

pub async fn send_mqtt_message(
    client: &mut MqttOperatorClient<tonic::transport::Channel>,
    payload: Vec<u8>,
    topic: &str,
) -> Result<String, String> {
    let _response = match client
        .send_mqtt_message(tonic::Request::new(MqttRequest {
            topic: topic.to_string(),
            message: payload,
        }))
        .await
    {
        Ok(res) => return Ok(res.into_inner().status),
        Err(e) => return Err(format!("Error:{}", e)),
    };
}

pub async fn receive_sensor_data(
    mut rx: mpsc::Receiver<sensor_grpc_adapter::ServerSensorChannel>,
) -> Result<(), String> {
    loop {
        info!("--- receive_sensor_data() ---");
        // Wait for Sensor Data from Sensor GRPC Service
        match rx.recv().await {
            Some(msg) => {
                // Connect to Database
                let db_client = db::establish_connection();
                info!("Sensor Data: {:?}", msg.data);
                // println!("{}", "-".repeat(20));
                // Retrieve Sensor ID (Sensor Table ID not Unique Sensor Identificator) from DB
                let _ = match db::select_sensor_by_name(&db_client, &msg.data.sensor_id) {
                    Ok(sensor) => {
                        info!("Sensor Selected with ID: {}", &msg.data.sensor_id);
                        // If Successful Parse Sensor Data and ...
                        let entry = db::SensorDataEntry {
                            sensor_id: sensor.id,
                            sensor_value: msg.data.value,
                            sensor_time: msg.data.timestamp as i64,
                            mqtt: false,
                            iota: false,
                            verified: false,
                        };
                        // Try making new DB Entry ...
                        match db::create_sensor_data(&db_client, entry) {
                            // On Success Positive GRPC Response
                            Ok(_) => {
                                info!("Sensor Data Saved to DB");
                                msg.tx.send(adapter::SensorReply {
                                    status: "Ok".to_string(),
                                    command: "".to_string(),
                                    payload: "".to_string(),
                                })
                            }
                            // On Failure Respond with Error
                            Err(_) => {
                                error!("Unable to Create DB Entry for Sensor Data");
                                msg.tx.send(adapter::SensorReply {
                                    status: "DB Error".to_string(),
                                    command: "".to_string(),
                                    payload: "".to_string(),
                                })
                            }
                        }
                    }
                    Err(_) => {
                        println!("Unable to Select Sensor with ID: {}", &msg.data.sensor_id);
                        msg.tx.send(adapter::SensorReply {
                            status: "Sensor Not Found".to_string(),
                            command: "".to_string(),
                            payload: "".to_string(),
                        })
                    }
                };
            }
            None => info!("No Sensor Data Available"),
        };
    }
}

pub fn generate_random_sequence() -> String {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    rand_string
}
