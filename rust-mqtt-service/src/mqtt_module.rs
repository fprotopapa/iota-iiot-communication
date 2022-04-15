use paho_mqtt as mqtt;
use std::env;
use tokio::time::{sleep, Duration};

use crate::config::{load_config_file, DEFAULT_SERVER_URL, ENV_SERVER_URL, MESSAGE_BUFFER_SIZE};
use crate::grpc_service::MqttOperatorService;
/// Struct to Hold References for GRPC Service
pub struct MqttHandler {
    pub mqtt_version: u32,
    pub qos_pub: i32,
    pub qos: Vec<i32>,
    pub topics: Vec<String>,
    pub host: String,
    pub tls_opt: paho_mqtt::SslOptions,
    pub recon_intv: (u64, u64),
    pub timeout: u64,
    pub keep_alive: u64,
}
/// Struct to Hold Message Information
pub struct MessageHandler<'a> {
    pub id: &'a String,
    pub pwd: &'a String,
    pub channel: &'a String,
    pub topic: &'a String,
    pub payload: &'a Vec<u8>,
}
/// GRPC Routine for Sending Messages
pub async fn send_grpc_message<'a>(
    cfg: &MqttOperatorService,
    msg: &'a MessageHandler<'a>,
) -> Result<String, String> {
    info!("--- send_grpc_message() ---");
    // Make Client
    let client_opt = create_client_option(&cfg.handler.host, msg.id, "pub");
    let client = create_client(client_opt)?;
    let conn_opt = create_conn_option(cfg, msg.id, msg.pwd, true);
    connect_to_broker(&client, conn_opt).await;
    let topic = make_topic_address(msg.channel, msg.topic);
    info!("Send Message to Topic: {}", topic);
    let status = send_message(&client, &topic, msg.payload.to_vec(), cfg.handler.qos_pub).await?;
    info!("{}", status);
    client.disconnect(None);
    Ok("Message Successfully Transmitted".to_string())
}
/// GRPC Routine for Receiving Messages
pub async fn receive_grpc_messages<'a>(
    cfg: &MqttOperatorService,
    msg: &'a MessageHandler<'a>,
    is_topic: bool,
) -> Result<(Vec<String>, Vec<Vec<u8>>), String> {
    info!("--- receive_grpc_message() ---");
    // Make Client
    let client_opt = create_client_option(&cfg.handler.host, msg.id, "sub");
    let mut client = create_client(client_opt)?;
    let conn_opt = create_conn_option(cfg, msg.id, msg.pwd, false);

    let msg_stream = client.get_stream(MESSAGE_BUFFER_SIZE);
    connect_to_broker(&client, conn_opt).await;
    if is_topic {
        client.subscribe(
            make_topic_address(msg.channel, msg.topic),
            cfg.handler.qos[0],
        );
    } else {
        let mut topics = Vec::new();
        for topic in cfg.handler.topics.clone() {
            topics.push(make_topic_address(msg.channel, &topic));
        }
        client.subscribe_many(&topics, &cfg.handler.qos);
    }
    let (subtopics, messages) = receive_messages(msg_stream).await;
    client.disconnect(None);

    Ok((subtopics, messages))
}
/// Build Client Options
fn create_client_option(host: &str, id: &str, postfix: &str) -> mqtt::CreateOptions {
    mqtt::CreateOptionsBuilder::new()
        .server_uri(host)
        .client_id(format!("{}_{}", id, postfix))
        .finalize()
}
/// Make Client from Option
pub fn create_client(client_opt: mqtt::CreateOptions) -> Result<paho_mqtt::AsyncClient, String> {
    match mqtt::AsyncClient::new(client_opt) {
        Ok(r) => return Ok(r),
        Err(e) => return Err(format!("Unable to Create MQTT Client: {}", e)),
    };
}
/// Build Connection Option
fn create_conn_option(
    cfg: &MqttOperatorService,
    id: &str,
    pwd: &str,
    clean_session: bool,
) -> paho_mqtt::ConnectOptions {
    // Connection Options
    // Set Clean Session
    mqtt::ConnectOptionsBuilder::new()
        .mqtt_version(cfg.handler.mqtt_version)
        .automatic_reconnect(
            Duration::from_secs(cfg.handler.recon_intv.0),
            Duration::from_secs(cfg.handler.recon_intv.1),
        )
        .keep_alive_interval(Duration::from_secs(cfg.handler.keep_alive))
        .connect_timeout(Duration::from_secs(cfg.handler.timeout))
        .ssl_options(cfg.handler.tls_opt.clone())
        .user_name(id)
        .password(pwd)
        .clean_session(clean_session)
        .finalize()
}
/// Build broker URL
pub fn create_broker_address(url: String, port: String, is_tls: bool) -> String {
    let ptc = if is_tls {
        "ssl".to_string()
    } else {
        "tcp".to_string()
    };
    format!("{}://{}:{}", ptc, url, port)
}
// Build Complete Topic
fn make_topic_address(channel_id: &str, topic: &str) -> String {
    format!("channels/{}/messages/{}", channel_id, topic)
}
/// Establish Connection to Broker
pub async fn connect_to_broker(client: &paho_mqtt::AsyncClient, conn_opt: mqtt::ConnectOptions) {
    while let Err(_) = client.connect(conn_opt.clone()).await {
        sleep(Duration::from_millis(1000)).await;
    }
}
/// Try Reconnect
#[allow(dead_code)]
pub async fn reconnect_to_broker(client: &paho_mqtt::AsyncClient) {
    while let Err(_) = client.reconnect().await {
        sleep(Duration::from_millis(1000)).await;
    }
}
/// Call to Send <essage
pub async fn send_message(
    client: &paho_mqtt::AsyncClient,
    topic: &str,
    msg: Vec<u8>,
    qos: i32,
) -> Result<String, String> {
    let payload = mqtt::Message::new(topic, msg, qos);
    let res = client.publish(payload).await;
    match res {
        Ok(_) => return Ok("Message Delivered".to_string()),
        Err(e) => return Err(format!("Unable to Send Message: {}", e)),
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
    info!("Number of Messages: {}", n_msgs);
    for _ in 0..n_msgs {
        match stream.recv().await {
            Ok(msg) => {
                if let Some(msg) = msg {
                    let topic: Vec<&str> = msg.topic().split("/").collect();
                    info!("Topic Received: {:?}", topic);
                    subtopics.push(if topic.len() > 3 {
                        topic[3].to_string()
                    } else {
                        topic[0].to_string()
                    });
                    messages.push(msg.payload().to_vec());
                    info!("Payload: {:?}", &msg.payload());
                } else {
                    error!("Lost connection.");
                    break;
                }
            }
            Err(e) => {
                error!("Unable to Read Message: {}", e);
                break;
            }
        }
    }
    (subtopics, messages)
}
/// Initialize MQTT Subscriber and Publisher, Load configuration from Config Files and Set-Up Communication
pub async fn init_controller() -> MqttHandler {
    info!("--- init_controller() ---");
    let cfg = load_config_file();
    // Build MQTT Server URL
    let url = env::var(ENV_SERVER_URL).unwrap_or(DEFAULT_SERVER_URL.to_string());
    let mqtt_host = create_broker_address(url.clone(), cfg.mqtt.port.to_string(), cfg.mqtt.tls);
    info!("MQTT Server Address: {}", &mqtt_host);
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
    // Assign to Handler
    MqttHandler {
        mqtt_version: mqtt_version,
        qos_pub: cfg.mqtt.qos_pub,
        qos: cfg.mqtt.qos,
        topics: cfg.mqtt.topics,
        host: mqtt_host,
        tls_opt: tls_opt,
        recon_intv: (cfg.mqtt.recon_intv.0, cfg.mqtt.recon_intv.1),
        timeout: cfg.mqtt.timeout,
        keep_alive: cfg.mqtt.keep_alive,
    }
}
