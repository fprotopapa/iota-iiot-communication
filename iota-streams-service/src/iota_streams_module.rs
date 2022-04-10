/// ENV name for Node URL for Tangle communication: STREAMS_NODE_URL
pub const ENV_NODE_URL: &str = "STREAMS_NODE_URL";
/// ENV name for local Proof of work setting: STREAMS_LOCAL_POW (default: false)
pub const ENV_LOCAL_POW: &str = "STREAMS_LOCAL_POW";
/// ENV name to set password for exporting author and subscriber state
pub const ENV_STATE_PWD: &str = "STREAMS_STATE_PWD";
/// Default valuefor node URL and password
pub const DEFAULT_NODE_URL: &str = "https://chrysalis-nodes.iota.org";
/// Default value for password to export states
pub const DEFAULT_STATE_PWD: &str = "123456";
/// Default value for folder name for saving exported states
pub const EXPORT_STATE_PATH: &str = "states";
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
    };
    use tokio::time::{sleep, Duration};

    pub async fn create_new_author(id: &str) -> String {
        let client = make_client().await;
        let cli = client.clone();
        println!("Make announcement");
        let mut author = make_author(cli);
        let ann_link = make_announcement(&mut author).await;
        match export_state(&mut author, id).await {
            Ok(_r) => return ann_link.to_string(),
            Err(_e) => return "".to_string(),
        };
    }

    pub async fn add_subscriber(id: &str, subscription_link: &str) -> String {
        let mut author = match import_state(id).await {
            Ok(aut) => aut,
            Err(_e) => return "".to_string(),
        };
        println!("Add subscriber");
        let sub_link = match parse_address(subscription_link) {
            Ok(address) => address,
            Err(e) => return e.to_string(),
        };
        receive_subscription(&mut author, &sub_link).await;
        let announcement_link = match author.announcement_link().clone() {
            Some(address) => address,
            None => return "".to_string(),
        };
        let (keyload_link, _) = make_keyload(&mut author, &announcement_link).await;
        match export_state(&mut author, id).await {
            Ok(_r) => return keyload_link.to_string(),
            Err(_e) => return "".to_string(),
        };
    }

    pub async fn send_message(id: &str, msg_link: &str, message: &str) -> String {
        let mut author = match import_state(id).await {
            Ok(aut) => aut,
            Err(_e) => return "".to_string(),
        };
        let msg_link = match parse_address(msg_link) {
            Ok(address) => address,
            Err(e) => return e.to_string(),
        };
        println!("Send message: {}", message);
        let (msg_link, _seq_link) = match author
            .send_signed_packet(
                &msg_link,
                &Bytes::default(),
                &Bytes(message.as_bytes().to_vec()),
            )
            .await
        {
            Ok(send) => send,
            Err(_e) => return "Error: Sending Message".to_string(),
        };
        println!(
            "Sent msg: {}, tangle index: {:#}",
            msg_link,
            msg_link.to_msg_index()
        );
        match export_state(&mut author, id).await {
            Ok(_r) => return msg_link.to_string(),
            Err(_e) => return msg_link.to_string(),
        };
    }

    pub fn make_author(client: Client) -> Author<Tangle> {
        let seed = generate_seed();
        let author = Author::new(&seed, ChannelType::SingleBranch, client);
        author
    }

    pub async fn make_announcement(author: &mut Author<Tangle>) -> Address {
        loop {
            let announcement_link = author.send_announce().await;
            match announcement_link {
                Ok(link) => {
                    println!("Announcement Link: {}", &link.to_string());
                    return link;
                }
                Err(e) => {
                    println!("{}", e);
                    sleep(Duration::from_millis(10000)).await;
                    continue;
                }
            }
        }
    }

    pub async fn make_keyload(
        author: &mut Author<Tangle>,
        announcement_link: &Address,
    ) -> (TangleAddress, Option<TangleAddress>) {
        loop {
            let res = author.send_keyload_for_everyone(announcement_link).await;
            match res {
                Ok(link) => {
                    println!("Keyload Link: {}", &link.0.to_string());
                    return link;
                }
                Err(e) => {
                    println!("{}", e);
                    sleep(Duration::from_millis(10000)).await;
                    continue;
                }
            }
        }
    }

    pub async fn receive_subscription(author: &mut Author<Tangle>, subscription_link: &Address) {
        loop {
            let res = author.receive_subscribe(subscription_link).await;
            match res {
                Ok(_r) => return (),
                Err(e) => {
                    println!("{}", e);
                    sleep(Duration::from_millis(10000)).await;
                    continue;
                }
            }
        }
    }

    pub async fn export_state(caller: &mut Author<Tangle>, id: &str) -> Result<(), ()> {
        let password = get_state_password();
        let path = get_state_path(id);
        let state = caller.export(&password).await;
        match state {
            Ok(state) => {
                let res = std::fs::write(path, state);
                match res {
                    Ok(_r) => return Ok(()),
                    Err(e) => {
                        println!("{}", e);
                        return Err(());
                    }
                }
            }
            Err(e) => {
                println!("{}", e);
                return Err(());
            }
        }
    }

    pub async fn import_state(id: &str) -> Result<Author<Tangle>, ()> {
        let password = get_state_password();
        let client = make_client().await;
        let path = get_state_path(id);
        let binary = std::fs::read(path);
        match binary {
            Ok(binary) => {
                let sub = Author::import(&binary, &password, client.clone()).await;
                match sub {
                    Ok(res) => return Ok(res),
                    Err(e) => {
                        println!("{}", e);
                        return Err(());
                    }
                }
            }
            Err(e) => {
                println!("{}", e);
                return Err(());
            }
        }
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
    use tokio::time::{sleep, Duration};

    pub async fn create_new_subscriber(id: &str, announcement_link: &str) -> String {
        let client = make_client().await;
        let mut subscriber = make_subscriber(client);
        let ann_link = match parse_address(announcement_link) {
            Ok(address) => address,
            Err(e) => return e.to_string(),
        };
        receive_announcement(&mut subscriber, &ann_link).await;
        let subscription_link = make_subscription(&mut subscriber, &ann_link).await;
        match export_state(&mut subscriber, id).await {
            Ok(_r) => return subscription_link.to_string(),
            Err(_e) => return "".to_string(),
        };
    }

    pub async fn receive_messages(id: &str) -> Vec<String> {
        let mut subscriber = match import_state(id).await {
            Ok(sub) => sub,
            Err(_e) => return vec!["".to_string()],
        };
        let messages = match subscriber.fetch_next_msgs().await {
            Ok(wrapped_msgs) => {
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
                msgs
            }
            Err(_e) => vec!["".to_string()],
        };
        match export_state(&mut subscriber, id).await {
            Ok(_r) => return messages,
            Err(_e) => return messages,
        };
    }

    pub async fn receive_keyload(id: &str, keyload_link: &str) -> String {
        let mut subscriber = match import_state(id).await {
            Ok(sub) => sub,
            Err(_e) => return "".to_string(),
        };
        let key_link = match parse_address(keyload_link) {
            Ok(address) => address,
            Err(e) => return e.to_string(),
        };
        match subscriber.receive_keyload(&key_link).await {
            Ok(res) => {
                if res {
                    match export_state(&mut subscriber, id).await {
                        Ok(_r) => return "Keyload processed".to_string(),
                        Err(_e) => return "".to_string(),
                    };
                } else {
                    return "".to_string();
                }
            }
            Err(_e) => return "".to_string(),
        };
    }

    pub fn make_subscriber(client: Client) -> Subscriber<Tangle> {
        let seed = generate_seed();
        let subscriber = Subscriber::new(&seed, client);
        subscriber
    }

    pub async fn receive_announcement(
        subscriber: &mut Subscriber<Tangle>,
        announcement_link: &Address,
    ) {
        loop {
            let res = subscriber.receive_announcement(announcement_link).await;
            match res {
                Ok(_r) => return (),
                Err(e) => {
                    println!("{}", e);
                    sleep(Duration::from_millis(10000)).await;
                    continue;
                }
            }
        }
    }

    pub async fn make_subscription(
        subscriber: &mut Subscriber<Tangle>,
        announcement_link: &Address,
    ) -> Address {
        loop {
            let subscription_link = subscriber.send_subscribe(announcement_link).await;
            match subscription_link {
                Ok(link) => return link,
                Err(e) => {
                    println!("{}", e);
                    sleep(Duration::from_millis(10000)).await;
                    continue;
                }
            }
        }
    }

    pub async fn export_state(caller: &mut Subscriber<Tangle>, id: &str) -> Result<(), ()> {
        let password = get_state_password();
        let path = get_state_path(id);
        let state = caller.export(&password).await;
        match state {
            Ok(state) => {
                let res = std::fs::write(path, state);
                match res {
                    Ok(_r) => return Ok(()),
                    Err(e) => {
                        println!("{}", e);
                        return Err(());
                    }
                }
            }
            Err(e) => {
                println!("{}", e);
                return Err(());
            }
        }
    }

    pub async fn import_state(id: &str) -> Result<Subscriber<Tangle>, ()> {
        let password = get_state_password();
        let client = make_client().await;
        let path = get_state_path(id);
        let binary = std::fs::read(path);
        match binary {
            Ok(binary) => {
                let sub = Subscriber::import(&binary, &password, client.clone()).await;
                match sub {
                    Ok(res) => return Ok(res),
                    Err(e) => {
                        println!("{}", e);
                        return Err(());
                    }
                }
            }
            Err(e) => {
                println!("{}", e);
                return Err(());
            }
        }
    }
}
/// util contains helper functions simplifying the use of
/// the IOTA Streams library for the Subscriber and Author instance.
///
pub mod util {
    use crate::iota_streams_module::{
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
            Err(_e) => return Err("".to_string()),
        }
    }

    pub async fn make_client() -> Client {
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
            Ok(r) => r,
            Err(_e) => panic!("Client build error."),
        };
        let client = Client::new(client_opt, iota_cli);
        client
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
