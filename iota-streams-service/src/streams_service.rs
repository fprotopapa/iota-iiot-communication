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
                        make_channel_reply(&request.id, request.msg_type, &ann_link, "Ok")
                    }
                    Err(e) => {
                        error!("{}", e);
                        make_channel_reply(&request.id, request.msg_type, "", &e)
                    }
                };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::CreateNewSubscriber => {
                let rep =
                    match streams_subscriber::create_new_subscriber(&request.id, &request.link)
                        .await
                    {
                        Ok(res) => {
                            let (sub_link, pk) = res;
                            ChannelReply {
                                id: request.id,
                                msg_type: request.msg_type,
                                link: sub_link,
                                status: "Ok".to_string(),
                                messages: None,
                                pk: Some(pk),
                            }
                        }
                        Err(e) => {
                            error!("{}", e);
                            make_channel_reply(&request.id, request.msg_type, "", &e)
                        }
                    };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::AddSubscriber => {
                let rep = match streams_author::add_subscriber(&request.id, &request.link).await {
                    Ok(key_link) => {
                        make_channel_reply(&request.id, request.msg_type, &key_link, "Ok")
                    }
                    Err(e) => {
                        error!("{}", e);
                        make_channel_reply(&request.id, request.msg_type, "", &e)
                    }
                };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::ReceiveKeyload => {
                let status =
                    match streams_subscriber::receive_keyload(&request.id, &request.link).await {
                        Ok(r) => r,
                        Err(e) => e,
                    };
                let _ = request.tx.send(QueueElem::Reply(make_channel_reply(
                    &request.id,
                    request.msg_type,
                    "",
                    &status,
                )));
            }

            MsgType::SendMessage => {
                let (payload, is_payload) = match request.messages {
                    Some(r) => (r[0].clone(), true),
                    None => ("No Payload".to_string(), false),
                };
                let rep = if !is_payload {
                    make_channel_reply(&request.id, request.msg_type, "", &payload)
                } else {
                    match streams_author::send_message(&request.id, &request.link, &payload).await {
                        Ok(msg_link) => {
                            make_channel_reply(&request.id, request.msg_type, &msg_link, "Ok")
                        }
                        Err(e) => {
                            error!("{}", e);
                            make_channel_reply(&request.id, request.msg_type, "", &e)
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
                            messages: Some(msgs),
                            pk: None,
                        }
                    }
                    Err(e) => {
                        error!("{}", e);
                        ChannelReply {
                            id: request.id,
                            msg_type: request.msg_type,
                            link: "".to_string(),
                            status: e,
                            messages: None,
                            pk: None,
                        }
                    }
                };
                let _ = request.tx.send(QueueElem::Reply(rep));
            }

            MsgType::RevokeAccess => {
                let pk = match request.pk {
                    Some(r) => r,
                    None => Vec::new(),
                };
                let status = match streams_author::remove_access(&request.id, &pk).await {
                    Ok(r) => r,
                    Err(e) => e,
                };
                let _ = request.tx.send(QueueElem::Reply(make_channel_reply(
                    &request.id,
                    request.msg_type,
                    "",
                    &status,
                )));
            }
            _ => error!("Error: Wrong Message Type"),
        }
        sleep(Duration::from_millis(2000)).await;
    }
}

fn make_channel_reply(id: &str, msg_type: MsgType, link: &str, status: &str) -> ChannelReply {
    ChannelReply {
        id: id.to_string(),
        msg_type: msg_type,
        link: link.to_string(),
        status: status.to_string(),
        messages: None,
        pk: None,
    }
}
