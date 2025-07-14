//! Encrypted connection example with Noise protocol
//!
//! This example demonstrates how to connect to an ESPHome device using
//! the Noise protocol for encrypted communication.
//!
//! Usage:
//! ```bash
//! cargo run --example noise_connection -- <host:port> <api_key>
//! # Example: cargo run --example noise_connection -- 192.168.1.100:6053 AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=
//! ```

use esphome_client::{types::DeviceInfoRequest, EspHomeClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    // Get address and API key from command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <host:port> <api_key>", args[0]);
        eprintln!(
            "Example: {} 192.168.1.100:6053 AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=",
            args[0]
        );
        eprintln!("\nNote: The API key is a 32-byte base64-encoded string.");
        eprintln!("You can find it in your ESPHome device's secrets.yaml or logs.");
        std::process::exit(1);
    }
    let address = &args[1];
    let api_key = &args[2];

    println!(
        "Connecting to ESPHome device at {} with encryption",
        address
    );

    // Build and connect with API key (enables Noise protocol encryption)
    let mut client = EspHomeClient::builder()
        .address(address)
        .key(api_key)
        .connect()
        .await?;

    println!("Connected successfully with encrypted channel!");

    // Request device information
    client.try_write(DeviceInfoRequest {}).await?;

    // Read responses
    loop {
        let response = client.try_read().await?;
        match response {
            esphome_client::types::EspHomeMessage::DeviceInfoResponse(device_info) => {
                println!("\n=== Device Information ===");
                println!("Name: {}", device_info.name);
                println!("Model: {}", device_info.model);
                println!("ESPHome Version: {}", device_info.esphome_version);
                break;
            }
            _ => {
                println!("Received unexpected message: {:?}", response);
            }
        }
    }

    // Close the connection gracefully
    client.close().await?;
    println!("\nSecure connection closed.");

    Ok(())
}
