// OPCUA for Rust
// SPDX-License-Identifier: MPL-2.0
// Copyright (C) 2017-2022 Adam Lock
// Copyright (C) 2022 Fabbio Protopapa

//! This is a simple server for OPC UA.
use rand::Rng;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use opcua_server::prelude::*;

mod config;
use config::load_config_file;
/// OPC-UA Server Implementation
fn main() {
    // Create an OPC UA server with sample configuration and default node set
    let cfg = load_config_file();
    let mut server =
        Server::new(ServerConfig::load(&PathBuf::from(cfg.opcua.serv_conf_path)).unwrap());

    let ns = {
        let address_space = server.address_space();
        let mut address_space = address_space.write().unwrap();
        address_space
            .register_namespace(&cfg.opcua.server_urn)
            .unwrap()
    };

    // Add some variables of our own
    add_sensor_variables(&mut server, ns, cfg.opcua.delay_ms);

    // Run the server. This does not ordinarily exit so you must Ctrl+C to terminate
    server.run();
}

/// Add sensor variables and update methods
fn add_sensor_variables(server: &mut Server, ns: u16, delay: u64) {
    // These will be the node ids of the new variables
    let t1_node = NodeId::new(ns, "t1");
    let t2_node = NodeId::new(ns, "t2");
    let t3_node = NodeId::new(ns, "t3");
    let t4_node = NodeId::new(ns, "t4");

    let address_space = server.address_space();

    // The address space is guarded so obtain a lock to change it
    {
        let mut address_space = address_space.write().unwrap();

        // Create a folder under objects folder
        let folder_id = address_space
            .add_folder("sensor", "sensor", &NodeId::objects_folder_id())
            .unwrap();

        // Add some variables to our folder. Values will be overwritten by the timer
        let _ = address_space.add_variables(
            vec![
                Variable::new(&t1_node, "t1", "t1", 0f64),
                Variable::new(&t2_node, "t2", "t2", 0f64),
                Variable::new(&t3_node, "t3", "t3", 0f64),
                Variable::new(&t4_node, "t4", "t4", 0f64),
            ],
            &folder_id,
        );
    }

    // This code will use a timer to set the values on variable t1, t2, t3 & t4.
    {
        // Store a counter and a flag in a tuple
        let data = Arc::new(Mutex::new((0f64, 0f64, 0f64, 0f64)));
        server.add_polling_action(delay, move || {
            let mut rng = rand::thread_rng();
            let mut data = data.lock().unwrap();
            data.0 = rng.gen_range(40.0..80.0);
            data.1 = rng.gen_range(30.0..90.0);
            data.2 = rng.gen_range(40.0..120.0);
            data.3 = rng.gen_range(10.0..70.0);
            let mut address_space = address_space.write().unwrap();
            let now = DateTime::now();
            let _ = address_space.set_variable_value(t1_node.clone(), data.0, &now, &now);
            let _ = address_space.set_variable_value(t2_node.clone(), data.1, &now, &now);
            let _ = address_space.set_variable_value(t3_node.clone(), data.2, &now, &now);
            let _ = address_space.set_variable_value(t4_node.clone(), data.3, &now, &now);
        });
    }
}
