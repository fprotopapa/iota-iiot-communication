#![warn(rust_2018_idioms)]
#[macro_use]
extern crate log;

use bytes::BytesMut;
use futures::stream::StreamExt;
use serde_json::Value;
use std::{io, str};
use tokio_serial::SerialPortBuilderExt;
use tokio_util::codec::{Decoder, Encoder};

use sensor_grpc_adapter as adapter;

mod config;
use config::load_config_file;

use std::time::SystemTime;
use tokio::time::{sleep, Duration};
/// Struct to Read Socket Data
struct LineCodec;
/// Decode Input
impl Decoder for LineCodec {
    type Item = String;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let newline = src.as_ref().iter().position(|b| *b == b'\n');
        if let Some(n) = newline {
            let line = src.split_to(n + 1);
            return match str::from_utf8(line.as_ref()) {
                Ok(s) => Ok(Some(s.to_string())),
                Err(_) => Err(io::Error::new(io::ErrorKind::Other, "Invalid String")),
            };
        }
        Ok(None)
    }
}
/// Encoder not used
impl Encoder<String> for LineCodec {
    type Error = io::Error;

    fn encode(&mut self, _item: String, _dst: &mut BytesMut) -> Result<(), Self::Error> {
        Ok(())
    }
}
/// Client Implementation Reading Sensor Data over Socket
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Start BLE Client");
    let cfg = load_config_file();
    // Initialize Connection
    let mut port = tokio_serial::new(cfg.ble.tty, 9600).open_native_async()?;
    let mut client = adapter::connect_sensor_adapter_client(&cfg.grpc.socket).await?;
    #[cfg(unix)]
    port.set_exclusive(false)
        .expect("Unable to set serial port exclusive to false");
    let mut reader = LineCodec.framed(port);
    // Wait for data
    loop {
        match reader.next().await {
            Some(line_result) => {
                let data = line_result.expect("Failed to read line");
                info!("Message: {}", data);
                let json_obj: Value = serde_json::from_str(&data)?;
                info!("Serial Read: {}", json_obj["temperature"]);
                let msg = make_temperature_message(&json_obj["temperature"].to_string()).await;
                match adapter::send_sensor_data(&mut client, msg).await {
                    Ok(r) => info!("Response T-Sensor: {}", r.status),
                    Err(e) => error!("Unable to Send Data: {}", e),
                };
            }
            None => (),
        }
        sleep(Duration::from_millis(cfg.ble.delay_ms)).await;
    }
}
/// Make SensorData Message
async fn make_temperature_message(value: &str) -> adapter::SensorData {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("Error Getting Time");
    adapter::SensorData {
        sensor_id: "12".to_string(),
        name: "MPC 7812".to_string(),
        sensor_type: "Temperature".to_string(),
        value: value.to_string(),
        unit: "C".to_string(),
        timestamp: timestamp.as_secs(),
        command: "".to_string(),
    }
}
