use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use crate::grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use crate::grpc_mqtt::MqttRequest;

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

pub fn generate_random_sequence() -> String {
    let rand_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    rand_string
}
