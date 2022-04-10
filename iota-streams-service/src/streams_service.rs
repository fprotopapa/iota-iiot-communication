use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};

use crate::grpc_service::{ChannelReply, QueueElem};
use crate::iota_streams_module::{streams_author, streams_subscriber};
use crate::msg_util::MsgType;
/// Implementation of streams state machine. Request are send over stable MPSC Channel,
/// reply to GRPC Server over Oneshot Channel per call.
pub async fn streams_state_machine(mut rx: mpsc::Receiver<QueueElem>) {
    loop {
        let request = match rx.recv().await {
            Some(msg) => match msg {
                QueueElem::Request(req) => req,
                _ => panic!("Received Wrong Data Type"),
            },
            None => panic!("Received No Data"),
        };
        match request.msg_type {
            MsgType::CreateNewAuthor => {
                let ann_link = streams_author::create_new_author(&request.id).await;
                let status = check_link(&ann_link);
                let _res = request.tx.send(QueueElem::Reply(ChannelReply {
                    id: request.id,
                    msg_type: request.msg_type,
                    link: ann_link,
                    status: status,
                    messages: None,
                }));
            }
            MsgType::CreateNewSubscriber => {
                let sub_link =
                    streams_subscriber::create_new_subscriber(&request.id, &request.link).await;
                let status = check_link(&sub_link);
                let _res = request.tx.send(QueueElem::Reply(ChannelReply {
                    id: request.id,
                    msg_type: request.msg_type,
                    link: sub_link,
                    status: status,
                    messages: None,
                }));
            }
            MsgType::AddSubscriber => {
                let key_link = streams_author::add_subscriber(&request.id, &request.link).await;
                let status = check_link(&key_link);
                let _res = request.tx.send(QueueElem::Reply(ChannelReply {
                    id: request.id,
                    msg_type: request.msg_type,
                    link: key_link,
                    status: status,
                    messages: None,
                }));
            }
            MsgType::ReceiveKeyload => {
                let link = streams_subscriber::receive_keyload(&request.id, &request.link).await;
                let status = check_link(&link);
                let _res = request.tx.send(QueueElem::Reply(ChannelReply {
                    id: request.id,
                    msg_type: request.msg_type,
                    link: "".to_string(),
                    status: status,
                    messages: None,
                }));
            }
            MsgType::SendMessage => {
                let msg_link = match request.messages.clone() {
                    Some(payload) => {
                        let msg_link =
                            streams_author::send_message(&request.id, &request.link, &payload[0])
                                .await;
                        msg_link
                    }
                    None => "".to_string(),
                };
                let status = check_link(&msg_link);
                let _res = request.tx.send(QueueElem::Reply(ChannelReply {
                    id: request.id,
                    msg_type: request.msg_type,
                    link: msg_link,
                    status: status,
                    messages: None,
                }));
            }
            MsgType::ReceiveMessages => {
                let recv_messages = streams_subscriber::receive_messages(&request.id).await;
                let status = if recv_messages.len() > 0 {
                    "Ok".to_string()
                } else {
                    "No messages".to_string()
                };
                let _res = request.tx.send(QueueElem::Reply(ChannelReply {
                    id: request.id,
                    msg_type: request.msg_type,
                    link: "".to_string(),
                    status: status,
                    messages: Some(recv_messages),
                }));
            }
            _ => println!("Error: Wrong Message Type"),
        }
        sleep(Duration::from_millis(2000)).await;
    }
}
/// Check for empty return. Error in stream calls is indicated through ""
fn check_link(link: &str) -> String {
    let status = if link.is_empty() {
        "Error".to_string()
    } else {
        "Ok".to_string()
    };
    status
}
