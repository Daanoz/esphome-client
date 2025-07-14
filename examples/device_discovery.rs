//! Device discovery example using mDNS
//!
//! This example demonstrates how to discover ESPHome devices on the local network
//! using mDNS service discovery.
//!
//! Requires the `discovery` feature to be enabled.
//!
//! Usage:
//! ```bash
//! cargo run --example device_discovery
//! ```

use esphome_client::discovery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    println!("=== ESPHome Device Discovery ===");
    println!("Searching for devices on the local network...\n");

    // Create a discovery client
    let mut discovered_devices = std::collections::HashSet::new();

    // Start discovery
    let mut result_stream = discovery::Client::default().discover()?;

    println!("Listening for device announcements (press Ctrl+C to stop)...\n");

    // Listen for discovered devices
    loop {
        match result_stream.next().await {
            Ok(device) => {
                // Check if we've seen this device before
                let device_id = format!(
                    "{}:{}",
                    device.hostname(),
                    device
                        .socket_address()
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                );

                if discovered_devices.insert(device_id) {
                    println!("=== New Device Discovered ===");
                    println!("Hostname: {}", device.hostname());

                    if let Some(addr) = device.socket_address() {
                        println!("Address: {}", addr);
                    }

                    println!(
                        "Encrypted: {}",
                        if device.has_encryption() { "Yes" } else { "No" }
                    );

                    println!("Attributes:");
                    for (key, value) in device.attributes() {
                        println!("  {} = {}", key, value);
                    }
                    println!();
                }
            }
            Err(e) => {
                eprintln!("Discovery error: {}", e);
                break;
            }
        }
    }

    println!("\nDiscovery stopped.");
    Ok(())
}
