use tokio::sync::mpsc;

use crate::db_module as db;
use sensor_grpc_adapter as adapter;

pub async fn receive_sensor_data(
    mut rx: mpsc::Receiver<sensor_grpc_adapter::ServerSensorChannel>,
) -> Result<(), String> {
    info!("--- receive_sensor_data() ---");
    loop {
        // Wait for Sensor Data from Sensor GRPC Service
        match rx.recv().await {
            Some(msg) => {
                // Connect to Database
                let db_client = db::establish_connection();
                info!("Sensor Data: {:?}", msg.data);
                // println!("{}", "-".repeat(20));
                // Retrieve Sensor ID (Sensor Table ID not Unique Sensor Identificator) from DB
                let _ = match db::select_sensor_by_name(&db_client, &msg.data.sensor_id) {
                    Ok(sensor) => {
                        info!("Sensor Selected with ID: {}", &msg.data.sensor_id);
                        // If Successful Parse Sensor Data and ...
                        let entry = db::SensorDataEntry {
                            sensor_id: sensor.id,
                            sensor_value: msg.data.value,
                            sensor_time: msg.data.timestamp as i64,
                            mqtt: false,
                            iota: false,
                            verified: false,
                        };
                        // Try making new DB Entry ...
                        match db::create_sensor_data(&db_client, entry) {
                            // On Success Positive GRPC Response
                            Ok(_) => {
                                info!("Sensor Data Saved to DB");
                                msg.tx.send(adapter::SensorReply {
                                    status: "Ok".to_string(),
                                    command: "".to_string(),
                                    payload: "".to_string(),
                                })
                            }
                            // On Failure Respond with Error
                            Err(_) => {
                                error!("Unable to Create DB Entry for Sensor Data");
                                msg.tx.send(adapter::SensorReply {
                                    status: "DB Error".to_string(),
                                    command: "".to_string(),
                                    payload: "".to_string(),
                                })
                            }
                        }
                    }
                    Err(_) => {
                        println!("Unable to Select Sensor with ID: {}", &msg.data.sensor_id);
                        msg.tx.send(adapter::SensorReply {
                            status: "Sensor Not Found".to_string(),
                            command: "".to_string(),
                            payload: "".to_string(),
                        })
                    }
                };
            }
            None => (),//info!("No Sensor Data Available"),
        };
    }
}
