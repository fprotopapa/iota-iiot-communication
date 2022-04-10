use paho_mqtt as mqtt;
use std::env;
use tokio::time::{sleep, Duration};

use crate::config::{
    load_config_file, DEFAULT_SERVER_URL, ENV_CHANNEL_ID, ENV_CLIENT_THING_ID,
    ENV_CLIENT_THING_PWD, ENV_SERVER_URL, ENV_THING_ID, ENV_THING_PWD,
};
/// Struct to Hold References for GRPC Service
pub struct MqttHandler {
    pub cli_pub: mqtt::AsyncClient,
    pub cli_sub: mqtt::AsyncClient,
    pub conn_opt_pub: mqtt::ConnectOptions,
    pub conn_opt_sub: mqtt::ConnectOptions,
    pub qos_pub: i32,
    pub qos: Vec<i32>,
    pub channel: String,
    pub topics: Vec<String>,
}
/// Make Client from Options
pub fn create_client(client_opt: mqtt::CreateOptions) -> paho_mqtt::AsyncClient {
    let client = mqtt::AsyncClient::new(client_opt).unwrap_or_else(|e| {
        panic!("Error creating the client: {:?}", e);
    });
    client
}
/// Build broker URL
pub fn create_broker_address(url: String, port: String, is_tls: bool) -> String {
    let ptc = if is_tls {
        "ssl".to_string()
    } else {
        "tcp".to_string()
    };
    let address = format!("{}://{}:{}", ptc, url, port);
    address
}
/// Establish Connection to Broker
pub async fn connect_to_broker(client: &paho_mqtt::AsyncClient, conn_opt: mqtt::ConnectOptions) {
    while let Err(_err) = client.connect(conn_opt.clone()).await {
        sleep(Duration::from_millis(1000)).await;
    }
}
/// Try Reconnect
#[allow(dead_code)]
pub async fn reconnect_to_broker(client: &paho_mqtt::AsyncClient) {
    while let Err(_err) = client.reconnect().await {
        sleep(Duration::from_millis(1000)).await;
    }
}
/// Call to Send <essage
pub async fn send_message(
    client: &paho_mqtt::AsyncClient,
    topic: &str,
    msg: Vec<u8>,
    qos: i32,
) -> String {
    let payload = mqtt::Message::new(topic, msg, qos);
    let res = client.publish(payload).await;
    match res {
        Ok(_r) => return "Ok".to_string(),
        Err(_e) => return "Didn't send message".to_string(),
    }
}
/// Call to Receive Messages
pub async fn receive_messages(
    stream: mqtt::AsyncReceiver<Option<paho_mqtt::Message>>,
) -> (Vec<String>, Vec<Vec<u8>>) {
    let mut messages = Vec::<Vec<u8>>::new();
    let mut subtopics = Vec::<String>::new();
    sleep(Duration::from_millis(1000)).await;
    stream.close();
    let n_msgs = stream.len();
    println!("Number of Messages: {}", n_msgs);
    for _ in 0..n_msgs {
        match stream.recv().await {
            Ok(msg) => {
                if let Some(msg) = msg {
                    let topic: Vec<&str> = msg.topic().split("/").collect();
                    println!("Topics: {:?}", topic);
                    subtopics.push(if topic.len() > 3 {
                        topic[3].to_string()
                    } else {
                        topic[0].to_string()
                    });
                    messages.push(msg.payload().to_vec());
                    println!("Payload: {:?}", &msg.payload());
                } else {
                    // A "None" means we were disconnected. Try to reconnect...
                    println!("Lost connection.");
                    break;
                }
            }
            Err(_e) => break,
        }
    }
    (subtopics, messages)
}
/// Initialize MQTT Subscriber and Publisher, Load configuration from Config Files and Set-Up Communication
pub async fn init_controller(is_server: bool) -> MqttHandler {
    let cfg = load_config_file();
    // Build MQTT Server URL
    let url = env::var(ENV_SERVER_URL).unwrap_or(DEFAULT_SERVER_URL.to_string());
    let mqtt_host = create_broker_address(url.clone(), cfg.mqtt.port.to_string(), cfg.mqtt.tls);
    println!("MQTT Server Address: {}", &mqtt_host);
    let channel_id = env::var(ENV_CHANNEL_ID).unwrap();
    println!("Channel ID: {}", &channel_id);
    // Load Secrets
    let (thing_id, thing_pwd) = if is_server {
        let id = env::var(ENV_THING_ID).unwrap();
        let pwd = env::var(ENV_THING_PWD).unwrap();
        (id, pwd)
    } else {
        let id = env::var(ENV_CLIENT_THING_ID).unwrap();
        let pwd = env::var(ENV_CLIENT_THING_PWD).unwrap();
        (id, pwd)
    };
    println!("Thing ID: {}", &thing_id);
    println!("Thing PWD: {}", &thing_pwd);
    // TLS Options
    let ca_cert = env::current_dir()
        .unwrap()
        .join("cert")
        .join(cfg.mqtt.ca_name);
    let tls_opt = mqtt::SslOptionsBuilder::new()
        .trust_store(ca_cert.clone())
        .unwrap()
        .finalize();
    // MQTT Version
    let mqtt_version = if cfg.mqtt.mqtt_v5 {
        mqtt::MQTT_VERSION_5
    } else {
        mqtt::MQTT_VERSION_3_1_1
    };
    // Client Options
    // Set Different Client ID (Otherwise Connection Problems)
    let client_opt_pub = mqtt::CreateOptionsBuilder::new()
        .server_uri(mqtt_host.clone())
        .client_id(format!("{}_pub", thing_id.clone()))
        .finalize();
    let client_opt_sub = mqtt::CreateOptionsBuilder::new()
        .server_uri(mqtt_host.clone())
        .client_id(format!("{}_sub", thing_id.clone()))
        .finalize();
    // Connection Options for Subscriber
    // Set Persistent Session
    let conn_opt_sub = mqtt::ConnectOptionsBuilder::new()
        .mqtt_version(mqtt_version)
        .automatic_reconnect(
            Duration::from_secs(cfg.mqtt.recon_intv.0),
            Duration::from_secs(cfg.mqtt.recon_intv.1),
        )
        .keep_alive_interval(Duration::from_secs(cfg.mqtt.keep_alive))
        .connect_timeout(Duration::from_secs(cfg.mqtt.timeout))
        .ssl_options(tls_opt.clone())
        .user_name(thing_id.clone())
        .password(thing_pwd.clone())
        .clean_session(cfg.mqtt.clean_session)
        .finalize();
    // Connection Options for Publisher
    // Set Clean Session
    let conn_opt_pub = mqtt::ConnectOptionsBuilder::new()
        .mqtt_version(mqtt_version)
        .automatic_reconnect(
            Duration::from_secs(cfg.mqtt.recon_intv.0),
            Duration::from_secs(cfg.mqtt.recon_intv.1),
        )
        .keep_alive_interval(Duration::from_secs(cfg.mqtt.keep_alive))
        .connect_timeout(Duration::from_secs(cfg.mqtt.timeout))
        .ssl_options(tls_opt.clone())
        .user_name(thing_id.clone())
        .password(thing_pwd.clone())
        .clean_session(true)
        .finalize();
    let client_pub = create_client(client_opt_pub);
    let client_sub = create_client(client_opt_sub);
    let mut topics: Vec<String> = Vec::<String>::new();
    // Build Topics
    for topic in cfg.mqtt.topics {
        topics.push(format!("channels/{}/messages/{}", channel_id, topic));
    }
    // Assign to Handler
    MqttHandler {
        cli_pub: client_pub,
        cli_sub: client_sub,
        conn_opt_pub: conn_opt_pub,
        conn_opt_sub: conn_opt_sub,
        qos_pub: cfg.mqtt.qos_pub,
        qos: cfg.mqtt.qos,
        channel: format!("channels/{}/messages/", channel_id),
        topics: topics,
    }
}
