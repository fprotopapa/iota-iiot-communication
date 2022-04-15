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
                let rep = match streams_author::create_new_author(&request.id).await {
                    Ok(ann_link) => {
                        make_channel_reply(&request.id, request.msg_type, &ann_link, "Ok", 0)
                    }
                    Err(e) => {
                        error!("{}", e);
                        make_channel_reply(&request.id, request.msg_type, "", &e, -1)
                    }
                };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::CreateNewSubscriber => {
                let rep =
                    match streams_subscriber::create_new_subscriber(&request.id, &request.link)
                        .await
                    {
                        Ok(sub_link) => {
                            make_channel_reply(&request.id, request.msg_type, &sub_link, "Ok", 0)
                        }
                        Err(e) => {
                            error!("{}", e);
                            make_channel_reply(&request.id, request.msg_type, "", &e, -1)
                        }
                    };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::AddSubscriber => {
                let (status, code) =
                    match streams_author::add_subscriber(&request.id, &request.link).await {
                        Ok(r) => (r, 0),
                        Err(e) => {
                            error!("{}", e);
                            (e, -1)
                        }
                    };
                let _ = request.tx.send(QueueElem::Reply(make_channel_reply(
                    &request.id,
                    request.msg_type,
                    "",
                    &status,
                    code,
                )));
            }

            MsgType::ReceiveKeyload => {
                let (status, code) =
                    match streams_subscriber::receive_keyload(&request.id, &request.link).await {
                        Ok(r) => (r, 0),
                        Err(e) => (e, -1),
                    };
                let _ = request.tx.send(QueueElem::Reply(make_channel_reply(
                    &request.id,
                    request.msg_type,
                    "",
                    &status,
                    code,
                )));
            }

            MsgType::SendMessage => {
                let (payload, is_payload) = match request.messages {
                    Some(r) => (r[0].clone(), true),
                    None => ("No Payload".to_string(), false),
                };
                let rep = if !is_payload {
                    make_channel_reply(&request.id, request.msg_type, "", &payload, -1)
                } else {
                    match streams_author::send_message(&request.id, &request.link, &payload).await {
                        Ok(msg_link) => {
                            make_channel_reply(&request.id, request.msg_type, &msg_link, "Ok", 0)
                        }
                        Err(e) => {
                            error!("{}", e);
                            make_channel_reply(&request.id, request.msg_type, "", &e, -1)
                        }
                    }
                };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::ReceiveMessages => {
                let rep = match streams_subscriber::receive_messages(&request.id).await {
                    Ok(msgs) => {
                        let status = if msgs.len() > 0 {
                            "Ok".to_string()
                        } else {
                            "No messages".to_string()
                        };
                        ChannelReply {
                            id: request.id,
                            msg_type: request.msg_type,
                            link: "".to_string(),
                            status: status,
                            code: 0,
                            messages: Some(msgs),
                        }
                    }
                    Err(e) => {
                        error!("{}", e);
                        ChannelReply {
                            id: request.id,
                            msg_type: request.msg_type,
                            link: "".to_string(),
                            status: e,
                            code: -1,
                            messages: None,
                        }
                    }
                };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::CreateKeyload => {
                let rep = match streams_author::announce_keyload(&request.id).await {
                    Ok(key_link) => {
                        make_channel_reply(&request.id, request.msg_type, &key_link, "Ok", 0)
                    }
                    Err(e) => {
                        error!("{}", e);
                        make_channel_reply(&request.id, request.msg_type, "", &e, -1)
                    }
                };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }
            _ => error!("Error: Wrong Message Type"),
        }
        sleep(Duration::from_millis(2000)).await;
    }
}

fn make_channel_reply(
    id: &str,
    msg_type: MsgType,
    link: &str,
    status: &str,
    code: i32,
) -> ChannelReply {
    ChannelReply {
        id: id.to_string(),
        msg_type: msg_type,
        link: link.to_string(),
        status: status.to_string(),
        code: code,
        messages: None,
    }
}
