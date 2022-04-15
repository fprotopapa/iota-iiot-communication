use serde_json::Value;
use std::env;
use std::path::{Path, PathBuf};

use identity::account::{Account, AccountStorage, IdentitySetup, Result};
use identity::core::{FromJson, ToJson, Url};
use identity::credential::{Credential, Subject};
use identity::crypto::SignatureOptions;
use identity::did::verifiable::VerifierOptions;
use identity::did::DID;
use identity::iota::{IotaDID, ResolvedIotaDocument, Resolver};

use crate::config::{
    load_config_file, save_config_file, DEFAULT_STRONGHOLD_PWD, ENV_STRONGHOLD_PWD,
    STRONGHOLD_FILE, STRONGHOLD_FOLDER,
};
/// Structure for exchanging data needed to make proofs,
/// verify and exchange identity information for requests
#[derive(Debug, Default)]
pub struct IdentityInformationRequest {
    pub did: String,
    pub challenge: String,
    pub verifiable_credential: String,
}
/// Structure for exchanging data needed to make proofs,
/// verify and exchange identity information for replies
#[derive(Debug, Default)]
pub struct IdentityInformationReply {
    pub did: String,
    pub challenge: String,
    pub verifiable_credential: String,
    pub status: String,
    pub code: i32,
}
/// Verifies verifiable credential signed with challenge.
/// Structure IdentityInformation needs did, challenge and VC to verifiy
/// Returns verification status ("Verified" or "Not Verified") or error
pub async fn verify_identity(
    request: IdentityInformationRequest,
) -> Result<IdentityInformationReply, String> {
    let did = parse_did(&request.did)?;
    // Make credential form string
    let credential = parse_vc(&request.verifiable_credential)?;
    // Fetch the DID Document from the Tangle
    let resolver: Resolver = match Resolver::new().await {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Fetch Document from Tangle: {}", e);
            return Err(format!("Unable to Fetch Document from Tangle: {}", e));
        }
    };
    let resolved: ResolvedIotaDocument = match resolver.resolve(&did).await {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Resolve Document: {}", e);
            return Err(format!("Unable to Resolve Document: {}", e));
        }
    };
    // Ensure the resolved DID Document can verify the credential signature
    let verified: bool = resolved
        .document
        .verify_data(
            &credential,
            &VerifierOptions {
                method_scope: None,
                method_type: None,
                challenge: Some(request.challenge),
                domain: None,
                purpose: None,
                allow_expired: None,
            },
        )
        .is_ok();
    let (status, code) = if verified {
        ("Identity Successfully Verified".to_string(), 0)
    } else {
        ("Unable to Verify Identity".to_string(), -1)
    };
    info!("DID: '{}' is Verified: {}", did, verified);
    Ok(IdentityInformationReply {
        did: request.did,
        challenge: "".to_string(),
        verifiable_credential: request.verifiable_credential,
        status: status,
        code: code,
    })
}
/// Generates verifiable credential signed with challenge.
/// Structure IdentityInformation needs did and challenge
/// Returns verification DID, Challenge (used to sign VC) and signed VC or error
pub async fn proof_identity(
    request: IdentityInformationRequest,
) -> Result<IdentityInformationReply, String> {
    let cfg = load_config_file();
    // Load account from disk
    let stronghold_path: PathBuf = Path::new(".").join(STRONGHOLD_FOLDER).join(STRONGHOLD_FILE);
    let password: String =
        env::var(ENV_STRONGHOLD_PWD).unwrap_or_else(|_| DEFAULT_STRONGHOLD_PWD.to_string());
    // Parse DID
    let did = parse_did(&request.did)?;
    let account: Account = match Account::builder()
        .storage(AccountStorage::Stronghold(
            stronghold_path,
            Some(password),
            None,
        ))
        .load_identity(did)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Import Account: {}", e);
            return Err(format!("Unable to Import Account: {}", e));
        }
    };
    // Load VC and convert to Credential
    let mut credential = parse_vc(&cfg.identity.verifiable_credential)?;
    // Sign the Credential with Challenge
    match account
        .sign(
            cfg.identity.fragment.as_str(),
            &mut credential,
            SignatureOptions {
                created: None,
                expires: None,
                challenge: Some(request.challenge.clone()),
                domain: None,
                purpose: None,
            },
        )
        .await
    {
        Ok(_) => info!(
            "Signed Verifiable Credential with Challenge: {}",
            &request.challenge
        ),
        Err(e) => {
            error!(
                "Unable to Sign VC with Challenge '{}': {}",
                &request.challenge, e
            );
            return Err(format!(
                "Unable to Sign VC with Challenge '{}': {}",
                &request.challenge, e
            ));
        }
    };
    let vc = credential_to_json(credential)?;
    Ok(IdentityInformationReply {
        did: request.did,
        challenge: request.challenge,
        verifiable_credential: vc,
        status: "Ok".to_string(),
        code: 0,
    })
}
/// Creates new identity.
/// Structure IdentityCreation needs device ID, Name and Type
/// Returns DID, unsigned VC (DID and VC also saved to config file) or error
pub async fn create_identity(vc: &str) -> Result<IdentityInformationReply, String> {
    let mut cfg = load_config_file();
    // Stronghold settings
    let stronghold_path: PathBuf = Path::new(".").join(STRONGHOLD_FOLDER).join(STRONGHOLD_FILE);
    let password: String =
        env::var(ENV_STRONGHOLD_PWD).unwrap_or_else(|_| DEFAULT_STRONGHOLD_PWD.to_string());
    // Create a new Account with stronghold storage.
    let mut account: Account = match Account::builder()
        .storage(AccountStorage::Stronghold(
            stronghold_path,
            Some(password),
            None,
        ))
        .create_identity(IdentitySetup::default())
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Create Account: {}", e);
            return Err(format!("Unable to Create Account: {}", e));
        }
    };
    // Add a new Ed25519 Verification Method to the identity
    match account
        .update_identity()
        .create_method()
        .fragment(cfg.identity.fragment.as_str())
        .apply()
        .await
    {
        Ok(_r) => info!("New Verification Method Added to Account"),
        Err(e) => {
            error!("Unable to Add Verification Method to Account: {}", e);
            return Err(format!(
                "Unable to Add Verification Method to Account: {}",
                e
            ));
        }
    };
    // Create a subject DID for the recipient of a `UniversityDegree` credential.
    let vc_json: Value = match serde_json::from_str(vc) {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Parse VC to JSON: {}", e);
            return Err(format!("Unable to Parse VC to JSON: {}", e));
        }
    };
    let subject: Subject = match Subject::from_json_value(vc_json) {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Parse VC to Subject: {}", e);
            return Err(format!("Unable to Parse VC to Subject: {}", e));
        }
    };
    // Issue an unsigned Credential...
    let did = match Url::parse(account.did().as_str()) {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Import DID From Account: {}", e);
            return Err(format!("Unable to Import DID From Account: {}", e));
        }
    };
    let credential: Credential = match Credential::builder(Default::default())
        .issuer(did.clone())
        .type_(cfg.identity.cred_type.as_str())
        .subject(subject)
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Create VC: {}", e);
            return Err(format!("Unable to Create VC: {}", e));
        }
    };
    let cred_json = credential_to_json(credential)?;
    // Save VC to file
    info!("Save DID: {} and VC {} to Config File", &did, &cred_json);
    cfg.identity.did = did.to_string();
    cfg.identity.verifiable_credential = cred_json.clone();
    save_config_file(cfg);

    Ok(IdentityInformationReply {
        did: did.to_string(),
        challenge: "".to_string(),
        verifiable_credential: cred_json,
        status: "Ok".to_string(),
        code: 0,
    })
}

fn credential_to_json(credential: Credential) -> Result<String, String> {
    match credential.to_json() {
        Ok(r) => return Ok(r),
        Err(e) => {
            error!("Unable to Parse Credential to JSON: {}", e);
            return Err(format!("Unable to Parse Credential to JSON: {}", e));
        }
    };
}

fn parse_did(did: &str) -> Result<IotaDID, String> {
    match IotaDID::parse(did) {
        Ok(res) => return Ok(res),
        Err(e) => {
            error!("Unable to Parse DID: {}", e);
            return Err(format!("Unable to Parse DID: {}", e));
        }
    };
}

fn parse_vc(vc: &str) -> Result<Credential, String> {
    match Credential::from_json(vc) {
        Ok(r) => return Ok(r),
        Err(e) => {
            error!("Unable to Parse VC from Request: {}", e);
            return Err(format!("Unable to Parse VC from Request: {}", e));
        }
    };
}
