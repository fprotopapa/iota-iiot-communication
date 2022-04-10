use grpc_mqtt::mqtt_operator_client::MqttOperatorClient;
use grpc_mqtt::MqttRequest;

/// Protobuffer v3 file
pub mod grpc_mqtt {
    tonic::include_proto!("mqtt_grpc");
}

mod config;
use config::load_config_file;
/// Client sends Message and Receives Message over MQTT Broker
/// mosquitto_pub -u <thing name> -P <thing pwd> -t channels/<channel id>/messages/did -h <host url> -p <port> --cafile ca.crt  -m '[{"bn":"test"}]' -q 1
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config_file();
    let mut client = MqttOperatorClient::connect(format!("http://{}", cfg.grpc.socket)).await?;
    // Make MQTT Message
    let payload: Vec<u8> = "Hello World".as_bytes().to_vec();
    let msg = MqttRequest {
        topic: "did".to_string(),
        message: payload,
    };
    let response = client.send_mqtt_message(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    println!("Send Message Status: {}", &response.status);
    println!("---------------------------------");
    let msg = MqttRequest {
        topic: "did".to_string(),
        message: vec![],
    };
    let response = client
        .receive_mqtt_message(tonic::Request::new(msg))
        .await?;
    let response = response.into_inner();
    for msg in response.messages {
        println!(
            "Message: {:?}",
            &String::from_utf8(msg).expect("Found invalid UTF-8")
        );
    }
    println!("---------------------------------");
    Ok(())
}
