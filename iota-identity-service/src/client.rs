use serde_json::json;

use grpc_identity::iota_identifier_client::IotaIdentifierClient;
use grpc_identity::{IotaIdentityCreationRequest, IotaIdentityRequest};

/// Protobuffer v3 file
pub mod grpc_identity {
    tonic::include_proto!("iota_identity_grpc");
}

mod config;
use crate::config::load_config_file;
/// Client implementation for Iota Identity GRPC Service
/// - Example creates an identity
/// - Signs verifiable credential with challenge
/// - Verifies signed credential
/// Communicatin between Client and Server via GRPC
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = load_config_file();
    let mut client = IotaIdentifierClient::connect(format!("http://{}", cfg.grpc.socket)).await?;
    // Create new identity
    let vc = json!({
        "Gateway": {
            "id": "12345",
            "name": "Sensor DHP"
        }
    });
    let msg = IotaIdentityCreationRequest {
        verifiable_credential: vc.to_string(),
    };
    let response = client.create_identity(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    let did = response.did;
    println!("New Identity with DID: {}", &did);
    println!("---------------------------------");
    // Sign verifiable credential with challenge
    let msg = IotaIdentityRequest {
        did: did,
        challenge: "abc-def-123".to_string(),
        verifiable_credential: "".to_string(),
    };
    let response = client.proof_identity(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    let did = response.did;
    let challenge = response.challenge;
    let vc = response.verifiable_credential;
    println!(
        "Signed Verifiable Credential with Challenge: {}",
        &challenge
    );
    println!("---------------------------------");
    println!("{}", &vc);
    println!("---------------------------------");
    // Verify signed credential
    let msg = IotaIdentityRequest {
        did: did,
        challenge: challenge,
        verifiable_credential: vc,
    };
    let response = client.verify_identity(tonic::Request::new(msg)).await?;
    let response = response.into_inner();
    println!("Identity is {}", response.status);

    Ok(())
}
