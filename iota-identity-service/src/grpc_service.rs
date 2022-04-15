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
        info!("create_new_identity: {:?}", request);
        let reply = match identity::create_identity(&request.verifiable_credential).await {
            Ok(r) => r,
            Err(e) => {
                return Err(Status::cancelled(format!(
                    "Unable to Create Identity: {}",
                    e
                )))
            }
        };

        Ok(Response::new(IotaIdentityReply {
            did: reply.did,
            challenge: reply.challenge,
            verifiable_credential: reply.verifiable_credential,
            status: reply.status,
            code: reply.code,
        }))
    }

    async fn verify_identity(
        &self,
        request: Request<IotaIdentityRequest>,
    ) -> Result<Response<IotaIdentityReply>, Status> {
        let request = request.into_inner();
        info!("verify_identity: {:?}", request);
        let reply = match identity::verify_identity(identity::IdentityInformationRequest {
            did: request.did,
            challenge: request.challenge,
            verifiable_credential: request.verifiable_credential,
        })
        .await
        {
            Ok(r) => r,
            Err(e) => {
                return Err(Status::cancelled(format!(
                    "Unable to Verify Identity: {}",
                    e
                )))
            }
        };
        Ok(Response::new(IotaIdentityReply {
            did: reply.did,
            challenge: reply.challenge,
            verifiable_credential: reply.verifiable_credential,
            status: reply.status,
            code: reply.code,
        }))
    }

    async fn proof_identity(
        &self,
        request: Request<IotaIdentityRequest>,
    ) -> Result<Response<IotaIdentityReply>, Status> {
        let request = request.into_inner();
        info!("proof_identity: {:?}", request);
        let reply = match identity::proof_identity(identity::IdentityInformationRequest {
            did: request.did,
            challenge: request.challenge,
            verifiable_credential: request.verifiable_credential,
        })
        .await
        {
            Ok(r) => r,
            Err(e) => return Err(Status::cancelled(format!("Unable to Create Proof: {}", e))),
        };
        Ok(Response::new(IotaIdentityReply {
            did: reply.did,
            challenge: reply.challenge,
            verifiable_credential: reply.verifiable_credential,
            status: reply.status,
            code: reply.code,
        }))
    }
}
