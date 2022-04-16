#[macro_use]
extern crate log;
use sensor_grpc_adapter as adapter;

mod config;
use config::{load_config_file, DEFAULT_BUFFER_SIZE};

use tokio::join;
use tokio::sync::mpsc;
/// Simple Server Implementation
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Mock Server");
    let cfg = load_config_file();
    let (service, rx) = adapter::SensorAdapterService::new(DEFAULT_BUFFER_SIZE);
    let sensor_worker = sensor_state_machine(rx);
    let grpc_server = adapter::run_sensor_adapter_server(service, &cfg.grpc.socket);
    let _result = join!(sensor_worker, grpc_server);
    Ok(())
}
/// Print Sensor Data and Echo Status back
async fn sensor_state_machine(mut rx: mpsc::Receiver<sensor_grpc_adapter::ServerSensorChannel>) {
    loop {
        let request = match rx.recv().await {
            Some(msg) => {
                println!("Data: {:?}", msg.data);
                println!("{}", "-".repeat(20));
                msg
            }
            None => panic!("Received No Data"),
        };
        let status = format!("Received: {} Data", &request.data.sensor_type);
        let _res = request.tx.send(adapter::SensorReply {
            status: status,
            command: "".to_string(),
            payload: "".to_string(),
        });
    }
}
