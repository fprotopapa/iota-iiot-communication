use grpc_mqtt::mqtt_operator_server::MqttOperator;
use grpc_mqtt::{MqttMsgsReply, MqttReply, MqttRequest};
use tonic::{Request, Response, Status};

use crate::mqtt_module::{
    init_controller, receive_grpc_messages, send_grpc_message, MessageHandler, MqttHandler,
};

/// Protobuffer v3 file
pub mod grpc_mqtt {
    tonic::include_proto!("mqtt_grpc");
}
/// Structure for Implementing GRPC Calls
pub struct MqttOperatorService {
    pub handler: MqttHandler,
}

/// Populate Struct with MQTT Settings
impl MqttOperatorService {
    pub async fn new() -> MqttOperatorService {
        let handler = init_controller().await;
        MqttOperatorService {
            handler: MqttHandler { ..handler },
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
        info!("send_mqtt_message()");
        let (status, code) = match send_grpc_message(
            &self,
            &MessageHandler {
                id: &request.id,
                pwd: &request.pwd,
                channel: &request.channel,
                topic: &request.topic,
                payload: &request.message,
            },
        )
        .await
        {
            Ok(r) => (r, 0),
            Err(e) => (e, -1),
        };
        Ok(Response::new(MqttReply {
            status: status,
            code: code,
        }))
    }

    async fn receive_mqtt_message(
        &self,
        request: Request<MqttRequest>,
    ) -> Result<Response<MqttMsgsReply>, Status> {
        let request = request.into_inner();
        info!("receive_mqtt_message()");
        // Check if Topic is set
        let is_topic = !request.topic.is_empty();

        let (subtopics, messages) = match receive_grpc_messages(
            &self,
            &MessageHandler {
                id: &request.id,
                pwd: &request.pwd,
                channel: &request.channel,
                topic: &request.topic,
                payload: &request.message,
            },
            is_topic,
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                error!("{}", e);
                return Err(Status::cancelled(format!(
                    "Error Receiving Messsages: {}",
                    e
                )));
            }
        };
        let (status, code) = if messages.is_empty() {
            ("No Messages".to_string(), 1)
        } else {
            ("Ok".to_string(), 0)
        };
        Ok(Response::new(MqttMsgsReply {
            topics: subtopics,
            messages: messages,
            status: status,
            code: code,
        }))
    }
}
