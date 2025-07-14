//! Unencrypted connection example
//!
//! This example demonstrates how to establish a basic connection to an ESPHome device
//! and retrieve device information.
//!
//! If the connect call fails with a "Connection reset by peer" error, it's probably
//! due to your esphome device requiring an encrypted connection.
//!
//! Usage:
//! ```bash
//! cargo run --example plain_connection -- <host:port>
//! # Example: cargo run --example plain_connection -- 192.168.1.100:6053
//! ```

use esphome_client::{types::DeviceInfoRequest, EspHomeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    // Get address from command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <host:port>", args[0]);
        eprintln!("Example: {} 192.168.1.100:6053", args[0]);
        std::process::exit(1);
    }
    let address = &args[1];

    println!("Connecting to ESPHome device at {}", address);

    // Build and connect to the ESPHome device
    let mut client = EspHomeClient::builder()
        .address(address)
        .connect()
        .await
        .inspect_err(|e| {
            if e.to_string().contains("Connection reset by peer") {
                tracing::warn!("Failed to setup connection, your device might be configured to use an encrypted connection")
            }
        })?;

    println!("Connected successfully!");

    // Request device information
    client.try_write(DeviceInfoRequest {}).await?;

    // Read responses
    loop {
        let response = client.try_read().await?;
        match response {
            esphome_client::types::EspHomeMessage::DeviceInfoResponse(device_info) => {
                println!("\n=== Device Information ===");
                println!("Name: {}", device_info.name);
                println!("Friendly Name: {}", device_info.friendly_name);
                println!("Model: {}", device_info.model);
                println!("Manufacturer: {}", device_info.manufacturer);
                println!("MAC Address: {}", device_info.mac_address);
                println!("ESPHome Version: {}", device_info.esphome_version);
                println!("Compilation Time: {}", device_info.compilation_time);
                println!("Has Deep Sleep: {}", device_info.has_deep_sleep);
                break;
            }
            _ => {
                println!("Received: {:?}", response);
            }
        }
    }

    // Close the connection gracefully
    client.close().await?;
    println!("\nConnection closed.");

    Ok(())
}
