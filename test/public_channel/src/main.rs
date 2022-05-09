#[macro_use]
extern crate log;

mod config;
use crate::config::load_config_file;
use crate::config::{DEFAULT_NODE_URL, ENV_LOCAL_POW, ENV_NODE_URL};
use iota_client::ClientBuilder;
use iota_streams::app::transport::tangle::TangleAddress;
use iota_streams::{
    app::transport::tangle::client::{Client, SendOptions},
    app_channels::api::tangle::{Address, Bytes, Subscriber},
    app_channels::Tangle,
};
use rand::Rng;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cfg = load_config_file();
    let client = make_client().await?;
    let seed = generate_seed();
    let mut subscriber = Subscriber::new(&seed, client);
    // Generate an Address object from the provided announcement link string from the Author
    let ann_address = parse_address(&cfg.announcement)?;
    // Receive the announcement message to start listening to the channel
    receive_announcement(&mut subscriber, &ann_address).await?;

    info!("Fetch Next Messages");
    let wrapped_msgs = match subscriber.fetch_next_msgs().await {
        Ok(r) => r,
        Err(e) => return Err(format!("Unable To Fetch Messages: {}", e).into()),
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
    for msg in msgs {
        info!("Message: {}", msg);
    }
    Ok(())
}

async fn make_client() -> Result<Client, String> {
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

fn make_client_opt(url: &str, local_pow: bool) -> SendOptions {
    SendOptions {
        url: url.to_string(),
        local_pow: local_pow,
    }
}

const ALPH9: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ9";

fn generate_seed() -> String {
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

fn parse_address(link: &str) -> Result<Address, String> {
    match link.parse::<TangleAddress>() {
        Ok(address) => return Ok(address),
        Err(e) => return Err(format!("Unable To Parse Address From String: {}", e)),
    }
}

async fn receive_announcement(
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
