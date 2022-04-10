use grpc_streams::iota_streamer_client::IotaStreamerClient;
use grpc_streams::{IotaStreamsRequest, IotaStreamsSendMessageRequest};

pub mod grpc_streams {
    tonic::include_proto!("iota_streams_grpc");
}

mod config;
use crate::config::load_config_file;

mod msg_util;
use msg_util::MsgType;
/// Client implementation for Iota Streams GRPC Service
/// - Example creates Author and Subscriber instance
/// - Adds subscriber via subscription link
/// - Sends keyload to subscriber
/// - Author sends signed test message over private Single Branch
/// - Subscriber receives message
/// Communicatin between Client and Server via GRPC
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config_file();
    let mut client = IotaStreamerClient::connect(format!("http://{}", cfg.socket)).await?;
    let author = "1".to_string();
    let subscriber = "2".to_string();
    // Create Author
    let msg = IotaStreamsRequest {
        id: author.clone(),
        msg_type: MsgType::CreateNewAuthor as u32,
        link: "".to_string(),
    };
    let response = client.create_new_author(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    let ann_link = response.link;
    println!("Received Announcement Link: {}", ann_link);
    // Create Subscriber
    let msg = IotaStreamsRequest {
        id: subscriber.clone(),
        msg_type: MsgType::CreateNewSubscriber as u32,
        link: ann_link,
    };
    let response = client
        .create_new_subscriber(tonic::Request::new(msg))
        .await?;
    let response = response.into_inner();
    let sub_link = response.link;
    println!("Received Subscription Link: {}", sub_link);
    // Author subscribe Subscriber to channel
    let msg = IotaStreamsRequest {
        id: author.clone(),
        msg_type: MsgType::AddSubscriber as u32,
        link: sub_link,
    };
    let response = client.add_subscriber(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    let key_link = response.link;
    println!("Received Keyload Link: {}", key_link);
    // Subscriber receive Keyload Link
    let msg = IotaStreamsRequest {
        id: subscriber.clone(),
        msg_type: MsgType::ReceiveKeyload as u32,
        link: key_link.clone(),
    };
    let response = client.receive_keyload(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    let status = response.status;
    println!("Subscriber received keyload link: {}", status);
    // Author send message
    let msg = "Importand message from author".to_string();
    let msg = IotaStreamsSendMessageRequest {
        id: author.clone(),
        msg_type: MsgType::SendMessage as u32,
        message_link: key_link,
        message_length: msg.len() as u32,
        message: msg,
    };
    let response = client.send_message(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    let status = response.status;
    let msg_link = response.link;
    println!("Send Message: Link: {}, Status: {}", msg_link, status);
    // Subscriber receiving messages
    let msg = IotaStreamsRequest {
        id: subscriber.clone(),
        msg_type: MsgType::ReceiveMessages as u32,
        link: "".to_string(),
    };
    let response = client.receive_messages(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    let status = response.status;
    let messages = response.messages;
    println!("Operation Status: {}", status);
    for msg in messages {
        println!("Received Message: {}", msg);
    }
    Ok(())
}
