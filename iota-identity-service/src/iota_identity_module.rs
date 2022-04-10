use std::env;
use std::path::{Path, PathBuf};

use identity::account::{Account, AccountStorage, IdentitySetup, Result};
use identity::core::{json, FromJson, ToJson, Url};
use identity::credential::{Credential, Subject};
use identity::crypto::{KeyPair, SignatureOptions};
use identity::did::verifiable::VerifierOptions;
use identity::did::DID;
use identity::iota::{IotaDID, ResolvedIotaDocument, Resolver};

use crate::config::{
    load_config_file, save_config_file, DEFAULT_STRONGHOLD_PWD, ENV_STRONGHOLD_PWD,
    STRONGHOLD_FILE, STRONGHOLD_FOLDER,
};
/// Structure for data needed to create new identity
#[derive(Debug, Default)]
pub struct IdentityCreation {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
}
/// Structure for exchanging data needed to make proofs,
/// verify and exchange identity information
#[derive(Debug, Default)]
pub struct IdentityInformation {
    pub did: String,
    pub challenge: String,
    pub verifiable_credential: String,
    pub status: String,
}
/// Verifies verifiable credential signed with challenge.
/// Structure IdentityInformation needs did, challenge and VC to verifiy
/// Returns verification status ("Verified" or "Not Verified") or error
pub async fn verify_identity(request: IdentityInformation) -> Result<IdentityInformation, String> {
    let mut reply = IdentityInformation::default();
    let did = match IotaDID::parse(&request.did) {
        Ok(res) => res,
        Err(e) => return Err(e.to_string()),
    };
    // Make credential form string
    let credential: Credential = match Credential::from_json(&request.verifiable_credential) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    // Fetch the DID Document from the Tangle
    let resolver: Resolver = match Resolver::new().await {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let resolved: ResolvedIotaDocument = match resolver.resolve(&did).await {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
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
    reply.status = if verified {
        "Verified".to_string()
    } else {
        "Not Verified".to_string()
    };
    println!("Credential Verified = {}", verified);
    Ok(reply)
}
/// Generates verifiable credential signed with challenge.
/// Structure IdentityInformation needs did and challenge
/// Returns verification DID, Challenge (used to sign VC) and signed VC or error
pub async fn proof_identity(request: IdentityInformation) -> Result<IdentityInformation, String> {
    let cfg = load_config_file();
    // Load account from disk
    let stronghold_path: PathBuf = Path::new(".").join(STRONGHOLD_FOLDER).join(STRONGHOLD_FILE);
    let password: String =
        env::var(ENV_STRONGHOLD_PWD).unwrap_or_else(|_| DEFAULT_STRONGHOLD_PWD.to_string());
    // Parse DID
    let did = match IotaDID::parse(request.did.as_str()) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
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
        Err(e) => return Err(e.to_string()),
    };

    // Load VC and convert to Credential
    let cred_json = cfg.identity.verifiable_credential;
    let mut credential: Credential = match Credential::from_json(&cred_json) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
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
        Ok(_r) => println!("Signed Verifiable Credential"),
        Err(e) => return Err(e.to_string()),
    };

    let vc = match credential.to_json() {
        Ok(res) => res,
        Err(e) => return Err(e.to_string()),
    };
    Ok(IdentityInformation {
        did: request.did,
        challenge: request.challenge,
        verifiable_credential: vc,
        status: "Ok".to_string(),
    })
}
/// Creates new identity.
/// Structure IdentityCreation needs device ID, Name and Type
/// Returns DID, unsigned VC (DID and VC also saved to config file) or error
pub async fn create_identity(request: IdentityCreation) -> Result<IdentityInformation, String> {
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
        Err(e) => return Err(e.to_string()),
    };

    // Add a new Ed25519 Verification Method to the identity
    match account
        .update_identity()
        .create_method()
        .fragment(cfg.identity.fragment.as_str())
        .apply()
        .await
    {
        Ok(_r) => println!("New Verification Method Added"),
        Err(e) => return Err(e.to_string()),
    };

    // Create a subject DID for the recipient of a `UniversityDegree` credential.
    let subject_key: KeyPair = match KeyPair::new_ed25519() {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let subject_did: IotaDID = match IotaDID::new(subject_key.public().as_ref()) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    // Create the actual Verifiable Credential subject.
    let subject: Subject = match Subject::from_json_value(json!({
    "id": subject_did.as_str(),
    request.device_type.as_str(): {
        "id": request.device_id.as_str(),
        "name": request.device_name.as_str()
    }
    })) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };

    // Issue an unsigned Credential...
    let did = match Url::parse(account.did().as_str()) {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let credential: Credential = match Credential::builder(Default::default())
        .issuer(did.clone())
        .type_(cfg.identity.cred_type.as_str())
        .subject(subject)
        .build()
    {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    let cred_json = match credential.to_json() {
        Ok(r) => r,
        Err(e) => return Err(e.to_string()),
    };
    // Save VC to file
    cfg.identity.did = did.to_string();
    cfg.identity.verifiable_credential = cred_json.clone();
    save_config_file(cfg);

    Ok(IdentityInformation {
        did: did.to_string(),
        challenge: "".to_string(),
        verifiable_credential: cred_json,
        status: "Ok".to_string(),
    })
}

// pub async update_identity() {
//     // Load account from disk
//     let account: Account = Account::builder()
//         .storage(AccountStorage::Stronghold(
//             stronghold_path,
//             Some(password),
//             None,
//         ))
//         .load_identity(did)
//         .await?;
//      let iota_did: &IotaDID = account.did();
//      let resolved: ResolvedIotaDocument = account.resolve_identity().await?;
//      // Prints the Identity Resolver Explorer URL.
//      // The entire history can be observed on this page by clicking "Loading History".
//      let explorer: &ExplorerUrl = ExplorerUrl::mainnet();
//      println!(
//          "[Example] Explore the DID Document = {}",
//          explorer.resolver_url(&did)?
//      );
// }
