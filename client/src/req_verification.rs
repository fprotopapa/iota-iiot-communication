use crate::config::TOPIC_DID;
use crate::db_module as db;
use crate::models::Identity;
use crate::mqtt_encoder as enc;
use crate::util::{connect_mqtt, generate_random_sequence, helper_send_mqtt, serialize_msg};

pub async fn request_identity_verification(channel_id: &str) -> Result<String, String> {
    info!("--- request_identity_verification() ---");
    // Connect to Database
    let db_client = db::establish_connection();
    // Connect to MQTT Service
    let mut mqtt_client = connect_mqtt().await?;
    let identities = get_identities(&db_client, false)?;
    for identity in identities {
        let payload = serialize_msg(&enc::Did {
            did: identity.did,
            challenge: generate_random_sequence(),
            vc: "".to_string(),
            proof: true,
        });
        helper_send_mqtt(&mut mqtt_client, payload, TOPIC_DID, channel_id).await?;
    }
    Ok("Verification Requests Send".to_string())
}

fn get_identities(
    db_client: &diesel::SqliteConnection,
    is_verified: bool,
) -> Result<Vec<Identity>, String> {
    match db::select_identities(&db_client, is_verified) {
        Ok(r) => {
            info!("Identities Selected with Entry Verified = {}", is_verified);
            return Ok(r);
        }
        Err(e) => {
            return Err(format!(
                "Unable to Select Identitites with Entry Verified = {}: Code: {}",
                is_verified, e
            ))
        }
    }
}
