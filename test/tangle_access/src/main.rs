#[macro_use]
extern crate log;

mod config;
use config::{load_config_file, save_config_file, STRONGHOLD_FOLDER, STRONGHOLD_FILE, DEFAULT_STRONGHOLD_PWD, ENV_STRONGHOLD_PWD};

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

/// Tokio runtime and start-up code for server implementation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    Ok(())
}

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
    save_config_file(cfg)?;

    Ok(IdentityInformationReply {
        did: did.to_string(),
        challenge: "".to_string(),
        verifiable_credential: cred_json,
        status: "Ok".to_string(),
        code: 0,
    })
}
