use grpc_identity::iota_identifier_server::IotaIdentifier;
use grpc_identity::{IotaIdentityCreationRequest, IotaIdentityReply, IotaIdentityRequest};
use tonic::{Request, Response, Status};

use crate::iota_identity_module as identity;

/// Protobuffer v3 file
pub mod grpc_identity {
    tonic::include_proto!("iota_identity_grpc");
}
/// Structure for Implementing GRPC Calls
#[derive(Debug, Default)]
pub struct IotaIdentityService {}
/// Implementation of GRPC Calls
/// create_identity, verify_identity, proof_identity
#[tonic::async_trait]
impl IotaIdentifier for IotaIdentityService {
    async fn create_identity(
        &self,
        request: Request<IotaIdentityCreationRequest>,
    ) -> Result<Response<IotaIdentityReply>, Status> {
        let request = request.into_inner();
        println!("create_new_identity: {:?}", request);
        let reply = match identity::create_identity(identity::IdentityCreation {
            device_id: request.device_id,
            device_name: request.device_name,
            device_type: request.device_type,
        })
        .await
        {
            Ok(r) => r,
            Err(e) => {
                println! {"{}", e};
                return Err(Status::cancelled("Identity Not Created"));
            }
        };

        Ok(Response::new(IotaIdentityReply {
            did: reply.did,
            challenge: reply.challenge,
            verifiable_credential: reply.verifiable_credential,
            status: reply.status,
        }))
    }

    async fn verify_identity(
        &self,
        request: Request<IotaIdentityRequest>,
    ) -> Result<Response<IotaIdentityReply>, Status> {
        let request = request.into_inner();
        println!("verify_identity: {:?}", request);
        let reply = match identity::verify_identity(identity::IdentityInformation {
            did: request.did,
            challenge: request.challenge,
            verifiable_credential: request.verifiable_credential,
            status: "".to_string(),
        })
        .await
        {
            Ok(r) => r,
            Err(e) => {
                println! {"{}", e};
                return Err(Status::cancelled("Error: Verification Not Completed"));
            }
        };
        Ok(Response::new(IotaIdentityReply {
            did: reply.did,
            challenge: reply.challenge,
            verifiable_credential: reply.verifiable_credential,
            status: reply.status,
        }))
    }

    async fn proof_identity(
        &self,
        request: Request<IotaIdentityRequest>,
    ) -> Result<Response<IotaIdentityReply>, Status> {
        let request = request.into_inner();
        println!("proof_identity: {:?}", request);
        let reply = match identity::proof_identity(identity::IdentityInformation {
            did: request.did,
            challenge: request.challenge,
            verifiable_credential: request.verifiable_credential,
            status: "".to_string(),
        })
        .await
        {
            Ok(r) => r,
            Err(e) => {
                println! {"{}", e};
                return Err(Status::cancelled("Error: Proof Not Completed"));
            }
        };
        Ok(Response::new(IotaIdentityReply {
            did: reply.did,
            challenge: reply.challenge,
            verifiable_credential: reply.verifiable_credential,
            status: reply.status,
        }))
    }
}
