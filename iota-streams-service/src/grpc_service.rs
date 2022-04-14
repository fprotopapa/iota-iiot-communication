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
    pub pk: Option<Vec<u8>>,
}
/// Reply Message, from State Machine to Server
#[derive(Debug)]
pub struct ChannelReply {
    pub id: String,
    pub msg_type: MsgType,
    pub status: String,
    pub link: String,
    pub messages: Option<Vec<String>>,
    pub pk: Option<Vec<u8>>,
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
/// receive_keyload, send_message, receive_messages, revoke_access
#[tonic::async_trait]
impl IotaStreamer for IotaStreamsService {
    async fn create_new_author(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        info!("create_new_author: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                    pk: Vec::new(),
                }))
            }
            Err(e) => return Err(Status::cancelled(format!("Author Not Generated: {}", e))),
        };
    }

    async fn create_new_subscriber(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        info!("create_new_subscriber: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                    pk: match response.pk {
                        Some(r) => r,
                        None => Vec::new(),
                    },
                }))
            }
            Err(e) => {
                return Err(Status::cancelled(format!(
                    "Subscriber Not Generated: {}",
                    e
                )))
            }
        };
    }

    async fn add_subscriber(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        info!("add_subscriber: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                    pk: Vec::new(),
                }))
            }
            Err(e) => return Err(Status::cancelled(format!("Subscriber Not Added: {}", e))),
        };
    }

    async fn receive_keyload(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        info!("receive_keyload: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                    pk: Vec::new(),
                }))
            }
            Err(e) => return Err(Status::cancelled(format!("Keyload Not Received: {}", e))),
        };
    }

    async fn send_message(
        &self,
        request: Request<IotaStreamsSendMessageRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        info!("send_message: {:?}", request);
        let (tx_one, rx_one) = oneshot::channel();
        let request = request.into_inner();
        let tx = self.tx.clone();
        match tx
            .send(QueueElem::Request(ChannelRequest {
                id: request.id,
                msg_type: convert_to_msgtype(request.msg_type),
                link: request.message_link,
                tx: tx_one,
                messages: Some(vec![request.message]),
                pk: Some(Vec::new()),
            }))
            .await
        {
            Ok(_) => (),
            Err(e) => return Err(Status::cancelled(format!("Keyload Not Received: {}", e))),
        };
        let response = match rx_one.await {
            Ok(resp) => match resp {
                QueueElem::Reply(resp) => resp,
                _ => {
                    return Err(Status::cancelled(
                        "Keyload Not Received: Wrong Data Structure Returned",
                    ))
                }
            },
            Err(e) => return Err(Status::cancelled(format!("Keyload Not Received: {}", e))),
        };

        return Ok(Response::new(IotaStreamsReply {
            id: response.id,
            msg_type: convert_from_msgtype(response.msg_type),
            link: response.link,
            status: response.status,
            pk: Vec::new(),
        }));
    }

    async fn receive_messages(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsRecvMessagesReply>, Status> {
        info!("receive_messages: {:?}", request);
        let (tx_one, rx_one) = oneshot::channel();
        let request = request.into_inner();
        let tx = self.tx.clone();
        match tx
            .send(QueueElem::Request(ChannelRequest {
                id: request.id,
                msg_type: convert_to_msgtype(request.msg_type),
                link: request.link,
                tx: tx_one,
                messages: None,
                pk: None,
            }))
            .await
        {
            Ok(_) => (),
            Err(e) => {
                return Err(Status::cancelled(format!(
                    "Error Receiving Messsages: {}",
                    e
                )))
            }
        };
        let response = match rx_one.await {
            Ok(resp) => match resp {
                QueueElem::Reply(resp) => resp,
                _ => {
                    return Err(Status::cancelled(
                        "Error Receiving Messsages: Wrong Data Structure Returned",
                    ))
                }
            },
            Err(e) => {
                return Err(Status::cancelled(format!(
                    "Error Receiving Messsages: {}",
                    e
                )))
            }
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

    async fn revoke_access(
        &self,
        request: Request<IotaStreamsRequest>,
    ) -> Result<Response<IotaStreamsReply>, Status> {
        info!("revoke_access: {:?}", request);
        match thread_communication(request, self.tx.clone()).await {
            Ok(response) => {
                return Ok(Response::new(IotaStreamsReply {
                    id: response.id,
                    msg_type: convert_from_msgtype(response.msg_type),
                    link: response.link,
                    status: response.status,
                    pk: Vec::new(),
                }))
            }
            Err(e) => return Err(Status::cancelled(format!("Access Not Removed: {}", e))),
        };
    }
}
/// Basic routine to poplate and distribute Request and Response
async fn thread_communication(
    request: Request<IotaStreamsRequest>,
    tx: mpsc::Sender<QueueElem>,
) -> Result<ChannelReply, String> {
    let (tx_one, rx_one) = oneshot::channel();
    let request = request.into_inner();
    match tx
        .send(QueueElem::Request(ChannelRequest {
            id: request.id,
            msg_type: convert_to_msgtype(request.msg_type),
            link: request.link,
            tx: tx_one,
            messages: None,
            pk: Some(request.pk),
        }))
        .await
    {
        Ok(_) => (),
        Err(e) => return Err(e.to_string()),
    };
    match rx_one.await {
        Ok(resp) => match resp {
            QueueElem::Reply(resp) => return Ok(resp),
            _ => return Err("Wrong Data Structure Returned".to_string()),
        },
        Err(e) => return Err(e.to_string()),
    };
}
