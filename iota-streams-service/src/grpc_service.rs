use tokio::sync::{mpsc, oneshot};
use tonic::{Request, Response, Status};

use grpc_streams::iota_streamer_server::IotaStreamer;
use grpc_streams::{
    IotaStreamsRecvMessagesReply, IotaStreamsReply, IotaStreamsRequest,
    IotaStreamsSendMessageRequest,
};

use crate::msg_util::{convert_from_msgtype, convert_to_msgtype, MsgType};
/// Protobuffer v3 file
pub mod grpc_streams {
    tonic::include_proto!("iota_streams_grpc");
}
/// Queue Element used for communication between GRPC Server and Streams State Machine
#[derive(Debug)]
pub enum QueueElem {
    Request(ChannelRequest),
    Reply(ChannelReply),
}
/// Request Message, from Server to State Machine
/// tx: Per call communication channel
#[derive(Debug)]
pub struct ChannelRequest {
    pub id: String,
    pub msg_type: MsgType,
    pub link: String,
    pub tx: oneshot::Sender<QueueElem>,
    pub messages: Option<Vec<String>>,
}
/// Reply Message, from State Machine to Server
#[derive(Debug)]
pub struct ChannelReply {
    pub id: String,
    pub msg_type: MsgType,
    pub status: String,
    pub link: String,
    pub messages: Option<Vec<String>>,
}
/// Structure for Implementing GRPC Calls,
/// tx: Stable MPSC Communication channel
#[derive(Debug)]
pub struct IotaStreamsService {
    tx: mpsc::Sender<QueueElem>,
}

impl IotaStreamsService {
    pub fn new(tx: mpsc::Sender<QueueElem>) -> IotaStreamsService {
        IotaStreamsService { tx: tx }
    }
}
/// Implementation of GRPC Calls
/// create_new_author, create_new_subscriber, add_subscriber,
/// receive_keyload, send_message, receive_messages
#[tonic::async_trait]
impl IotaStreamer for IotaStreamsService {
    async fn create_new_author(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        println!("create_new_author: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                }))
            }
            Err(_e) => return Err(Status::cancelled("Author Not Generated")),
        };
    }

    async fn create_new_subscriber(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        println!("create_new_subscriber: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                }))
            }
            Err(_e) => return Err(Status::cancelled("Subscriber Not Generated")),
        };
    }

    async fn add_subscriber(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        println!("add_subscriber: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                }))
            }
            Err(_e) => return Err(Status::cancelled("Subscriber Not Added")),
        };
    }

    async fn receive_keyload(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        println!("receive_keyload: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                }))
            }
            Err(_e) => return Err(Status::cancelled("Keyload Not Received")),
        };
    }

    async fn send_message(
        &self,
        request: Request<IotaStreamsSendMessageRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        let (tx_one, rx_one) = oneshot::channel();
        let request = request.into_inner();
        let tx = self.tx.clone();
        let _res = tx
            .send(QueueElem::Request(ChannelRequest {
                id: request.id,
                msg_type: convert_to_msgtype(request.msg_type),
                link: request.message_link,
                tx: tx_one,
                messages: Some(vec![request.message]),
            }))
            .await;
        let response = match rx_one.await {
            Ok(resp) => match resp {
                QueueElem::Reply(resp) => resp,
                _ => return Err(Status::cancelled("Keyload Not Received")),
            },
            Err(_) => return Err(Status::cancelled("Keyload Not Received")),
        };

        return Ok(Response::new(IotaStreamsReply {
            id: response.id,
            msg_type: convert_from_msgtype(response.msg_type),
            link: response.link,
            status: response.status,
        }));
    }

    async fn receive_messages(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsRecvMessagesReply>, Status> {
        let (tx_one, rx_one) = oneshot::channel();
        let request = request.into_inner();
        let tx = self.tx.clone();
        let _res = tx
            .send(QueueElem::Request(ChannelRequest {
                id: request.id,
                msg_type: convert_to_msgtype(request.msg_type),
                link: request.link,
                tx: tx_one,
                messages: None,
            }))
            .await;
        let response = match rx_one.await {
            Ok(resp) => match resp {
                QueueElem::Reply(resp) => resp,
                _ => return Err(Status::cancelled("Error Receiving Messsages")),
            },
            Err(_) => return Err(Status::cancelled("Error Receiving Messsages")),
        };
        let messages = match response.messages {
            Some(msgs) => msgs,
            None => vec![],
        };
        return Ok(Response::new(IotaStreamsRecvMessagesReply {
            id: response.id,
            msg_type: convert_from_msgtype(response.msg_type),
            link: response.link,
            status: response.status,
            received_messages: messages.len() as u32,
            messages: messages,
        }));
    }
}
/// Basic routine to poplate and distribute Request and Response
async fn thread_communication(
    request: Request<IotaStreamsRequest>,
    tx: mpsc::Sender<QueueElem>,
) -> Result<ChannelReply, ()> {
    let (tx_one, rx_one) = oneshot::channel();
    let request = request.into_inner();
    let _res = tx
        .send(QueueElem::Request(ChannelRequest {
            id: request.id,
            msg_type: convert_to_msgtype(request.msg_type),
            link: request.link,
            tx: tx_one,
            messages: None,
        }))
        .await;

    let response = match rx_one.await {
        Ok(resp) => match resp {
            QueueElem::Reply(resp) => resp,
            _ => return Err(()),
        },
        Err(_) => return Err(()),
    };
    Ok(response)
}
