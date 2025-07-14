//! Sensor monitoring example
//!
//! This example demonstrates how to subscribe to sensor updates from an ESPHome device
//! and continuously monitor their values.
//!
//! Usage:
//! ```bash
//! cargo run --example sensor_monitoring -- <host:port> [api_key]
//! # Example: cargo run --example sensor_monitoring -- 192.168.1.100:6053
//! ```

use esphome_client::{
    types::{EspHomeMessage, ListEntitiesRequest, SubscribeStatesRequest},
    EspHomeClient,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    // Get address and optional API key from command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <host:port> [api_key]", args[0]);
        eprintln!("Example: {} 192.168.1.100:6053", args[0]);
        std::process::exit(1);
    }
    let address = &args[1];
    let api_key = args.get(2);

    println!("Connecting to ESPHome device at {}", address);

    // Build connection with optional API key
    let mut builder = EspHomeClient::builder().address(address);
    if let Some(key) = api_key {
        builder = builder.key(key);
    }
    let mut client = builder.connect().await?;

    println!("Connected successfully!");

    // Store sensor information
    let mut sensors: HashMap<u32, String> = HashMap::new();

    // Request list of all entities
    client.try_write(ListEntitiesRequest {}).await?;

    // Subscribe to state updates
    client.try_write(SubscribeStatesRequest {}).await?;

    println!("\n=== Monitoring Sensors ===");
    println!("Press Ctrl+C to stop\n");

    // Read and process messages
    loop {
        let response = client.try_read().await?;
        match response {
            EspHomeMessage::ListEntitiesSensorResponse(sensor) => {
                // Store sensor information
                sensors.insert(sensor.key, sensor.name.clone());
                println!(
                    "[Sensor Discovered] {} - {} ({})",
                    sensor.name, sensor.unit_of_measurement, sensor.device_class
                );
            }
            EspHomeMessage::SensorStateResponse(state) => {
                // Display sensor state update
                if let Some(sensor_name) = sensors.get(&state.key) {
                    if state.missing_state {
                        println!("[{}] State missing", sensor_name);
                    } else {
                        println!("[{}] {:.2}", sensor_name, state.state);
                    }
                } else {
                    println!("[Unknown Sensor {}] {:.2}", state.key, state.state);
                }
            }
            EspHomeMessage::ListEntitiesDoneResponse(_) => {
                println!("\n--- All entities listed ---\n");
            }
            _ => {
                // Ignore other message types
            }
        }
    }
}
