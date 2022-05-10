#[macro_use]
extern crate log;
use chrono;
use serde_json::json;
use std::sync::{Arc, Mutex};

mod config;
use config::{load_config_file, save_config_file, ENV_TEST_IDENTITIES};

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
    let counter = Arc::new(Mutex::new(0));
    let is_test = env_to_bool(ENV_TEST_IDENTITIES);
    let mut handles = Vec::new();
    if !is_test {
        for i in 0..100 {
            let counter = Arc::clone(&counter);
            match create_identity(format!("identity_{}", i), counter).await {
                Ok(r) => info!("{}", r),
                Err(e) => error!("{}", e),
            };
        }
    } else {
        for i in 0..100 {
            let counter = Arc::clone(&counter);
            let handle = tokio::spawn(verify_identity(format!("identity_{}", i), counter));
            handles.push(handle);
        }
        for handle in handles {
            let _ = handle.await.unwrap();
        }
    }
    info!(
        "------------------ Time: {}, Total: {} ------------------",
        chrono::offset::Local::now(),
        *counter.lock().unwrap()
    );
    Ok(())
}

async fn create_identity(filename: String, counter: Arc<Mutex<i32>>) -> Result<String, String> {
    let vc = json!({
        "device": {
            "type": "Device",
            "id": filename
        }
    })
    .to_string();
    // Stronghold settings
    let stronghold_path: PathBuf = Path::new(".").join(format!("strong_{}.hodl", filename));
    let password: String = "123456".to_string();
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
        .fragment("dev-1")
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
    let vc_json: Value = match serde_json::from_str(&vc) {
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
    let mut credential: Credential = match Credential::builder(Default::default())
        .issuer(did.clone())
        .type_("Device Identification")
        .subject(subject)
        .build()
    {
        Ok(r) => r,
        Err(e) => {
            error!("Unable to Create VC: {}", e);
            return Err(format!("Unable to Create VC: {}", e));
        }
    };
    // Sign the Credential with Challenge
    match account
        .sign(
            "dev-1",
            &mut credential,
            SignatureOptions {
                created: None,
                expires: None,
                challenge: None,
                domain: None,
                purpose: None,
            },
        )
        .await
    {
        Ok(_) => info!("Signed Verifiable Credential"),
        Err(e) => {
            error!("Unable to Sign VC: {}", e);
            return Err(format!("Unable to Sign VC: {}", e));
        }
    };
    let cred_json = credential_to_json(credential)?;
    // Save VC to file
    let mut cfg = load_config_file(&filename);
    cfg.identity.did = did.to_string();
    cfg.identity.vc = cred_json.clone();
    info!("Save DID: {} and VC {} to Config File", &did, &cred_json);
    save_config_file(cfg, &filename)?;
    let mut num = counter.lock().unwrap();
    *num += 1;
    Ok("Identity Created".to_string())
}

/// Verifies verifiable credential signed with challenge.
/// Structure IdentityInformation needs did, challenge and VC to verifiy
/// Returns verification status ("Verified" or "Not Verified") or error
async fn verify_identity(filename: String, counter: Arc<Mutex<i32>>) -> Result<String, String> {
    let cfg = load_config_file(&filename);
    let did = parse_did(&cfg.identity.did)?;
    // Make credential form string
    let credential = parse_vc(&cfg.identity.vc)?;
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
                challenge: None,
                domain: None,
                purpose: None,
                allow_expired: None,
            },
        )
        .is_ok();

    info!("DID: '{}' is Verified: {}", did, verified);
    let mut num = counter.lock().unwrap();
    *num += 1;
    Ok("Identity Verified".to_string())
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

fn env_to_bool(env_name: &str) -> bool {
    let bool_str = env::var(env_name).unwrap_or_else(|_| "true".to_string());
    let is_bool: bool = match bool_str.as_str() {
        "true" => true,
        "t" => true,
        "false" => false,
        "f" => false,
        _ => false,
    };
    is_bool
}
