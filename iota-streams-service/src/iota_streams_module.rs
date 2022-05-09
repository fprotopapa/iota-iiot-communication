/// streams_author contains helper functions simplifying the use of
/// the IOTA Streams library for the Author instance. Communication via Public and Private Single Branch.
///
/// API Calls with a high-level of abstraction are:
/// - create_new_author(id: &str) -> String:
///     Generates a new author instance, send an announcement message to the tangle
///     and exports the instance to ./EXPORT_STATE_PATH/<id>
///     Returns the announcement link as a string
///
/// - add_subscriber(id: &str, subscription_link: &str) -> String
///     Author adding Subscriber through subscription link, Author
///     instance imported from disk and afterwards exported with altert state
///     Returns keyload link as string
///  
/// - send_message(id: &str, msg_link: &str, message: &str) -> String
///     Author sending signed message to tangle. Author
///     instance imported from disk and afterwards exported with altert state
///     Returns next message link as string
///
pub mod streams_author {
    use crate::iota_streams_module::util::{
        generate_seed, get_state_password, get_state_path, make_client, parse_address,
    };
    use iota_streams::{
        app::transport::tangle::{client::Client, TangleAddress},
        app_channels::api::tangle::{Address, Author, Bytes, ChannelType},
        app_channels::Tangle,
        core_edsig::signature::ed25519::PublicKey,
    };

    pub async fn create_new_author(id: &str) -> Result<String, String> {
        let client = make_client().await?;
        info!("Create New Channel Through Announcement");
        let mut author = make_author(client);
        let ann_link = make_announcement(&mut author).await?;
        export_state(&mut author, id).await?;
        Ok(ann_link.to_string())
    }

    pub async fn add_subscriber(id: &str, subscription_link: &str) -> Result<String, String> {
        let mut author = import_state(id).await?;
        let sub_link = parse_address(subscription_link)?;
        info!(
            "Add Subscriber with Subscription Link: {}",
            subscription_link
        );
        receive_subscription(&mut author, &sub_link).await?;
        export_state(&mut author, id).await?;
        Ok("Subscriber Succesfully Added".to_string())
    }

    pub async fn announce_keyload(id: &str) -> Result<String, String> {
        let mut author = import_state(id).await?;
        let announcement_link = match author.announcement_link().clone() {
            Some(address) => address,
            None => return Err("No Announcement Link Found".to_string()),
        };
        let (keyload_link, _) = make_keyload(&mut author, &announcement_link).await?;
        export_state(&mut author, id).await?;
        Ok(keyload_link.to_string())
    }

    pub async fn remove_access(id: &str, pk: &Vec<u8>) -> Result<String, String> {
        let mut author = import_state(id).await?;
        let public_key = match PublicKey::from_bytes(pk) {
            Ok(res) => res,
            Err(e) => return Err(format!("Unable to Convert String to Public Key: {}", e)),
        };
        // info!("Public Key to Remove: {:?}", public_key);
        match author.remove_subscriber(public_key) {
            Ok(_) => {
                info!("Successfully Removed Subscriber");
                export_state(&mut author, id).await?;
                return Ok("Successfully Removed Subscriber".to_string());
            }
            Err(e) => {
                error!("Unable to Remove Subscriber: {}", e);
                return Err(format!("Unable to Remove Subscriber: {}", e));
            }
        };
    }

    pub async fn send_message(id: &str, msg_link: &str, message: &str) -> Result<String, String> {
        let mut author = import_state(id).await?;
        let msg_link = parse_address(msg_link)?;
        info!("Send message: {}", message);
        let (msg_link, _seq_link) = match author
            .send_signed_packet(
                &msg_link,
                &Bytes::default(),
                &Bytes(message.as_bytes().to_vec()),
            )
            .await
        {
            Ok(send) => {
                info!(
                    "Sent msg: {}, tangle index: {:#}",
                    msg_link,
                    msg_link.to_msg_index()
                );
                send
            }
            Err(e) => return Err(format!("Error: Sending Message: {}", e)),
        };
        export_state(&mut author, id).await?;
        Ok(msg_link.to_string())
    }

    pub fn make_author(client: Client) -> Author<Tangle> {
        let seed = generate_seed();
        let author = Author::new(&seed, ChannelType::SingleBranch, client);
        author
    }

    pub async fn make_announcement(author: &mut Author<Tangle>) -> Result<Address, String> {
        match author.send_announce().await {
            Ok(link) => {
                info!("Announcement Link: {}", &link.to_string());
                return Ok(link);
            }
            Err(e) => {
                error!("{}", e);
                return Err(format!("Unable to Make Announcement: {}", e));
            }
        };
    }

    pub async fn make_keyload(
        author: &mut Author<Tangle>,
        announcement_link: &Address,
    ) -> Result<(TangleAddress, Option<TangleAddress>), String> {
        match author.send_keyload_for_everyone(announcement_link).await {
            Ok(link) => {
                info!("Keyload Link: {}", &link.0.to_string());
                return Ok(link);
            }
            Err(e) => {
                error!("{}", e);
                return Err(format!("Unable to Make Keyload: {}", e));
            }
        };
    }

    pub async fn receive_subscription(
        author: &mut Author<Tangle>,
        subscription_link: &Address,
    ) -> Result<(), String> {
        match author.receive_subscribe(subscription_link).await {
            Ok(_) => {
                info!("Author Received Subscription");
                return Ok(());
            }
            Err(e) => {
                error!("{}", e);
                return Err(format!("Unable to Receive Subscription: {}", e));
            }
        };
    }

    pub async fn export_state(caller: &mut Author<Tangle>, id: &str) -> Result<(), String> {
        let password = get_state_password();
        let path = get_state_path(id);
        match caller.export(&password).await {
            Ok(state) => {
                match std::fs::write(path, state) {
                    Ok(_) => {
                        info!("State Successfully Exported");
                        return Ok(());
                    }
                    Err(e) => return Err(format!("Unable To Write State: {}", e)),
                };
            }
            Err(e) => return Err(format!("Unable To Export State: {}", e)),
        };
    }

    pub async fn import_state(id: &str) -> Result<Author<Tangle>, String> {
        let password = get_state_password();
        let client = make_client().await?;
        let path = get_state_path(id);
        let binary = match std::fs::read(path) {
            Ok(r) => r,
            Err(e) => return Err(format!("Unable To Read State: {}", e)),
        };
        match Author::import(&binary, &password, client).await {
            Ok(r) => {
                info!("State Successfully Imported");
                return Ok(r);
            }
            Err(e) => return Err(format!("Unable To Import State: {}", e)),
        };
    }
}
/// streams_subscriber contains helper functions simplifying the use of
/// the IOTA Streams library for the Subscriber instance.
///
/// API Calls with a high-level of abstraction are:
/// - create_new_subscriber(id: &str, announcement_link: &str) -> String:
///     Generates a new subscriber instance and subscripes to channel via announcement link
///     and exports the instance to ./EXPORT_STATE_PATH/<id>
///     Returns the subscription link as a string
///
/// - receive_messages(id: &str) -> Vec<String>
///     Subscriber receiving messages send from author. Subscriber
///     instance imported from disk and afterwards exported with altert state
///     Returns received messages as vector of strings
///  
/// - receive_keyload(id: &str, keyload_link: &str) -> String
///     Subscriber receiving keyload link. Subscriber
///     instance imported from disk and afterwards exported with altert state
///     Returns information stating success or failure
///
pub mod streams_subscriber {
    use crate::iota_streams_module::util::{
        generate_seed, get_state_password, get_state_path, make_client, parse_address,
    };
    use iota_streams::{
        app::transport::tangle::client::Client,
        app_channels::api::tangle::{Address, Bytes, Subscriber},
        app_channels::Tangle,
    };
    pub async fn create_new_subscriber(
        id: &str,
        announcement_link: &str,
    ) -> Result<String, String> {
        let client = make_client().await?;
        let mut subscriber = make_subscriber(client);
        let ann_link = parse_address(announcement_link)?;
        receive_announcement(&mut subscriber, &ann_link).await?;
        let subscription_link = make_subscription(&mut subscriber, &ann_link).await?;
        export_state(&mut subscriber, id).await?;
        Ok(subscription_link.to_string())
    }

    pub async fn receive_messages(id: &str) -> Result<Vec<String>, String> {
        let mut subscriber = import_state(id).await?;
        info!("Fetch Next Messages");
        let wrapped_msgs = match subscriber.fetch_next_msgs().await {
            Ok(r) => r,
            Err(e) => return Err(format!("Unable To Fetch Messages: {}", e)),
        };
        let mut msgs = Vec::new();
        for msg in wrapped_msgs {
            msgs.push(
                msg.body
                    .masked_payload()
                    .and_then(Bytes::as_str)
                    .unwrap_or("None")
                    .to_string(),
            )
        }
        export_state(&mut subscriber, id).await?;
        Ok(msgs)
    }

    pub async fn receive_keyload(id: &str, keyload_link: &str) -> Result<String, String> {
        let mut subscriber = import_state(id).await?;
        let key_link = parse_address(keyload_link)?;
        let is_received = match subscriber.receive_keyload(&key_link).await {
            Ok(r) => r,
            Err(e) => return Err(format!("Subscriber Unable To Receive Keyload: {}", e)),
        };
        if is_received {
            info!("Subscriber Received Keyload");
            export_state(&mut subscriber, id).await?;
            return Ok("Subscriber Received Keyload".to_string());
        } else {
            return Err("Subscriber Unable To Receive Keyload".to_string());
        }
    }

    pub fn make_subscriber(client: Client) -> Subscriber<Tangle> {
        let seed = generate_seed();
        let subscriber = Subscriber::new(&seed, client);
        subscriber
    }

    pub async fn receive_announcement(
        subscriber: &mut Subscriber<Tangle>,
        announcement_link: &Address,
    ) -> Result<(), String> {
        match subscriber.receive_announcement(announcement_link).await {
            Ok(_r) => {
                info!("Subscriber Received Announcement Link");
                return Ok(());
            }
            Err(e) => {
                error!("{}", e);
                return Err(format!("Unable to Receive Announcement: {}", e));
            }
        };
    }

    pub async fn make_subscription(
        subscriber: &mut Subscriber<Tangle>,
        announcement_link: &Address,
    ) -> Result<Address, String> {
        match subscriber.send_subscribe(announcement_link).await {
            Ok(link) => {
                info!("Subscription Link: {}", &link.to_string());
                return Ok(link);
            }
            Err(e) => {
                error!("{}", e);
                return Err(format!("Unable to Make Subscription: {}", e));
            }
        };
    }

    pub async fn export_state(caller: &mut Subscriber<Tangle>, id: &str) -> Result<(), String> {
        let password = get_state_password();
        let path = get_state_path(id);
        let state = match caller.export(&password).await {
            Ok(r) => r,
            Err(e) => return Err(format!("Unable to Write State: {}", e)),
        };
        match std::fs::write(path, state) {
            Ok(_) => {
                info!("State Successfully Exported");
                return Ok(());
            }
            Err(e) => return Err(format!("Unable to Export State: {}", e)),
        }
    }

    pub async fn import_state(id: &str) -> Result<Subscriber<Tangle>, String> {
        let password = get_state_password();
        let client = make_client().await?;
        let path = get_state_path(id);
        let binary = match std::fs::read(path) {
            Ok(r) => r,
            Err(e) => return Err(format!("Unable to Read State: {}", e)),
        };
        match Subscriber::import(&binary, &password, client).await {
            Ok(r) => {
                info!("State Successfully Imported");
                return Ok(r);
            }
            Err(e) => return Err(format!("Unable to Import State: {}", e)),
        };
    }
}
/// util contains helper functions simplifying the use of
/// the IOTA Streams library for the Subscriber and Author instance.
///
pub mod util {
    use crate::config::{
        DEFAULT_NODE_URL, DEFAULT_STATE_PWD, ENV_LOCAL_POW, ENV_NODE_URL, ENV_STATE_PWD,
        EXPORT_STATE_PATH,
    };
    use iota_client::ClientBuilder;
    use iota_streams::app::transport::tangle::client::{Client, SendOptions};
    use iota_streams::app::transport::tangle::TangleAddress;
    use iota_streams::app_channels::api::tangle::Address;
    use rand::Rng;
    use std::env;
    use std::path::Path;

    pub const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";

    pub fn generate_seed() -> String {
        let seed: String = (0..81)
            .map(|_| {
                ALPH9
                    .chars()
                    .nth(rand::thread_rng().gen_range(0..27))
                    .unwrap()
            })
            .collect::<String>();
        seed
    }

    pub fn get_state_path(filename: &str) -> std::path::PathBuf {
        let path = Path::new(".").join(EXPORT_STATE_PATH).join(filename);
        path
    }

    pub fn get_state_password() -> String {
        let password = env::var(ENV_STATE_PWD).unwrap_or_else(|_| DEFAULT_STATE_PWD.to_string());
        password
    }

    pub fn parse_address(link: &str) -> Result<Address, String> {
        match link.parse::<TangleAddress>() {
            Ok(address) => return Ok(address),
            Err(e) => return Err(format!("Unable To Parse Address From String: {}", e)),
        }
    }

    pub async fn make_client() -> Result<Client, String> {
        let node_url = env::var(ENV_NODE_URL).unwrap_or_else(|_| DEFAULT_NODE_URL.to_string());
        let local_pow = match env::var(ENV_LOCAL_POW)
            .unwrap_or_else(|_| "false".to_string())
            .as_str()
        {
            "true" => true,
            "false" => false,
            _ => false,
        };
        let client_opt = make_client_opt(&node_url, local_pow);
        let iota_cli = match ClientBuilder::new()
            .with_node(&node_url)
            .unwrap()
            .with_local_pow(local_pow)
            .finish()
            .await
        {
            Ok(r) => {
                info!("Streams Client Created");
                r
            }
            Err(e) => return Err(format!("Unable to Create Streams Client: {}", e)),
        };
        let client = Client::new(client_opt, iota_cli);
        Ok(client)
    }

    pub fn make_client_opt(url: &str, local_pow: bool) -> SendOptions {
        SendOptions {
            url: url.to_string(),
            local_pow: local_pow,
        }
    }
}

#[cfg(test)]
mod tests {
    // cargo test -- --nocapture //for std out
    use super::*;
    #[test]
    fn test_generate_seed() {
        let samples = 100;
        let mut seeds: Vec<String> = vec![String::new(); samples];
        let mut len = 0;
        for _ in 1..=samples {
            let seed = util::generate_seed();
            println!("{}: {}", len, &seed);
            len += 1;
            seeds.push(seed);
        }
        seeds.sort();
        seeds.dedup();
        let mut len = 0;
        for seed in &seeds {
            if !seed.is_empty() {
                println!("{}: {}", len, seed);
                len += 1;
            }
        }
        assert_eq!(len, samples);
    }
}
