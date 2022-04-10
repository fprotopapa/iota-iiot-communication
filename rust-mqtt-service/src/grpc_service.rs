use grpc_mqtt::mqtt_operator_server::MqttOperator;
use grpc_mqtt::{MqttMsgsReply, MqttReply, MqttRequest};
use paho_mqtt::{AsyncClient, ConnectOptions};
use tonic::{Request, Response, Status};

use crate::mqtt_module::{connect_to_broker, init_controller, receive_messages, send_message};

use crate::config::MESSAGE_BUFFER_SIZE;

/// Protobuffer v3 file
pub mod grpc_mqtt {
    tonic::include_proto!("mqtt_grpc");
}
/// Structure for Implementing GRPC Calls
pub struct MqttOperatorService {
    pub cli_pub: AsyncClient,
    pub cli_sub: AsyncClient,
    pub conn_opt_pub: ConnectOptions,
    pub conn_opt_sub: ConnectOptions,
    pub qos_pub: i32,
    pub qos: Vec<i32>,
    pub channel: String,
    pub topics: Vec<String>,
}
/// Populate Struct with MQTT Settings
impl MqttOperatorService {
    pub async fn new(is_server: bool) -> MqttOperatorService {
        let handler = init_controller(is_server).await;
        MqttOperatorService {
            cli_pub: handler.cli_pub,
            cli_sub: handler.cli_sub,
            conn_opt_pub: handler.conn_opt_pub,
            conn_opt_sub: handler.conn_opt_sub,
            qos_pub: handler.qos_pub,
            qos: handler.qos,
            channel: handler.channel,
            topics: handler.topics,
        }
    }
}
/// Implementation of GRPC Calls
/// send_mqtt_message, receive_mqtt_message
#[tonic::async_trait]
impl MqttOperator for MqttOperatorService {
    async fn send_mqtt_message(
        &self,
        request: Request<MqttRequest>,
    ) -> Result<Response<MqttReply>, Status> {
        let request = request.into_inner();
        println!("send_mqtt_message: {:?}", request);
        let client = self.cli_pub.clone();
        connect_to_broker(&client, self.conn_opt_pub.clone()).await;
        let topic = format!("{}{}", &self.channel, request.topic);
        println!("Send to topic: {}", &topic);
        let status = send_message(&client, &topic, request.message, self.qos_pub).await;
        client.disconnect(None);
        Ok(Response::new(MqttReply { status: status }))
    }

    async fn receive_mqtt_message(
        &self,
        request: Request<MqttRequest>,
    ) -> Result<Response<MqttMsgsReply>, Status> {
        let request = request.into_inner();
        println!("receive_mqtt_message: {:?}", request);
        let mut client = self.cli_sub.clone();
        let msg_stream = client.get_stream(MESSAGE_BUFFER_SIZE);
        connect_to_broker(&client, self.conn_opt_sub.clone()).await;
        client.subscribe_many(&self.topics, &self.qos);
        let (subtopics, messages) = receive_messages(msg_stream).await;
        client.disconnect(None);
        println!("Subtopics: {:?}", subtopics);
        println!("Messages: {:?}", messages);
        if !messages.is_empty() {
            Ok(Response::new(MqttMsgsReply {
                topics: subtopics,
                messages: messages,
                status: "Ok".to_string(),
            }))
        } else {
            Ok(Response::new(MqttMsgsReply {
                topics: subtopics,
                messages: messages,
                status: "No Messages".to_string(),
            }))
        }
    }
}
