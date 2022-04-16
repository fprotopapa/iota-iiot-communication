#[macro_use]
extern crate log;
use std::time::SystemTime;
use std::{
    path::PathBuf,
    sync::{mpsc, Arc, Mutex, RwLock},
};

use sensor_grpc_adapter as adapter;

mod config;
use config::load_config_file;

use opcua_client::prelude::*;

/// Client OPC-UA Implementation, Subscribe to OPC-UA Server and Send Received Data over GRPC
#[tokio::main]
async fn main() -> Result<(), ()> {
    env_logger::init();
    info!("OPC-UA Client");
    let cfg = load_config_file();
    let endpoint_id = &cfg.opcua.endpoint;
    // Enter Main Thread, Start GRPC Communication
    let (tx, rx) = mpsc::channel::<(NodeId, DataValue)>();
    tokio::spawn(async move {
        // Set-Up Connection
        let mut client = match adapter::connect_sensor_adapter_client(&cfg.grpc.socket).await {
            Ok(r) => r,
            Err(_e) => panic!("Unable to Create GRPC Client"),
        };
        // Wait for Message from Callback and Process
        loop {
            let (node_id, data_value) = rx.recv().unwrap();
            let value = if let Some(ref value) = data_value.value {
                value.to_string()
            } else {
                "null".to_string()
            };
            let msg = build_message(node_id.identifier, node_id.namespace, &value);
            // Send Data over GRPC
            let _res = adapter::send_sensor_data(&mut client, msg).await;
        }
    });
    // Use the sample client config to set up a client. The sample config has a number of named
    // endpoints one of which is marked as the default.
    let mut client =
        Client::new(ClientConfig::load(&PathBuf::from(cfg.opcua.cli_conf_path)).unwrap());
    let endpoint_id: Option<&str> = if !endpoint_id.is_empty() {
        Some(&endpoint_id)
    } else {
        None
    };
    let ns = 2;
    if let Ok(session) = client.connect_to_endpoint_id(endpoint_id) {
        let _ = subscription_loop(session, tx, ns).map_err(|err| {
            println!("ERROR: Got an error while performing action - {}", err);
        });
    }
    Ok(())
}
/// Register Callback and Subscribe to OPC-UA Server
fn subscription_loop(
    session: Arc<RwLock<Session>>,
    tx: mpsc::Sender<(NodeId, DataValue)>,
    ns: u16,
) -> Result<(), StatusCode> {
    // Create a subscription
    println!("Creating subscription");
    // This scope is important - we don't want to session to be locked when the code hits the
    // loop below
    {
        let mut session = session.write().unwrap();
        // Creates our subscription - one update every two seconds. The update is sent as a message
        // to the main thread and published over GRPC.
        let tx = Arc::new(Mutex::new(tx));
        let subscription_id = session.create_subscription(
            2000f64,
            10,
            30,
            0,
            0,
            true,
            DataChangeCallback::new(move |items| {
                println!("Data change from server:");
                let tx = tx.lock().unwrap();
                items.iter().for_each(|item| {
                    let node_id = item.item_to_monitor().node_id.clone();
                    let value = item.value().clone();
                    let _ = tx.send((node_id, value));
                });
            }),
        )?;
        println!("Created a subscription with id = {}", subscription_id);
        // Create some monitored items
        let items_to_create: Vec<MonitoredItemCreateRequest> = ["t1", "t2", "t3", "t4"]
            .iter()
            .map(|v| NodeId::new(ns, *v).into())
            .collect();
        let _ = session.create_monitored_items(
            subscription_id,
            TimestampsToReturn::Both,
            &items_to_create,
        )?;
    }
    // Loops forever. The publish thread will call the callback with changes on the variables
    let _ = Session::run(session);

    Ok(())
}

fn build_message(id: node_id::Identifier, namespace: u16, value: &str) -> adapter::SensorData {
    // Build Identifier
    let identifier = format!("{}{}", namespace, id);
    println!("ID: {}, Value: {}", &identifier, &value);
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error Getting Time");
    match id.to_string().as_str() {
        "s=t1" => adapter::SensorData {
            sensor_id: identifier,
            name: "OPC-UA MCP12".to_string(),
            sensor_type: "Temperature".to_string(),
            value: value.to_string(),
            unit: "C".to_string(),
            timestamp: timestamp.as_secs(),
            command: "".to_string(),
        },
        "s=t2" => adapter::SensorData {
            sensor_id: identifier,
            name: "OPC-UA NTC-1280".to_string(),
            sensor_type: "Temperature".to_string(),
            value: value.to_string(),
            unit: "C".to_string(),
            timestamp: timestamp.as_secs(),
            command: "".to_string(),
        },
        "s=t3" => adapter::SensorData {
            sensor_id: identifier,
            name: "OPC-UA MCP18".to_string(),
            sensor_type: "Temperature".to_string(),
            value: value.to_string(),
            unit: "C".to_string(),
            timestamp: timestamp.as_secs(),
            command: "".to_string(),
        },
        "s=t4" => adapter::SensorData {
            sensor_id: identifier,
            name: "OPC-UA EYK56".to_string(),
            sensor_type: "Temperature".to_string(),
            value: value.to_string(),
            unit: "C".to_string(),
            timestamp: timestamp.as_secs(),
            command: "".to_string(),
        },
        _ => adapter::SensorData {
            sensor_id: "None".to_string(),
            name: "Error".to_string(),
            sensor_type: "Temperature".to_string(),
            value: "None".to_string(),
            unit: "-".to_string(),
            timestamp: timestamp.as_secs(),
            command: "".to_string(),
        },
    }
}
