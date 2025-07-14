//! Switch control example
//!
//! This example demonstrates how to discover switches on an ESPHome device
//! and control them (turn on/off).
//!
//! Usage:
//! ```bash
//! cargo run --example switch_control -- <host:port> [api_key]
//! # Example: cargo run --example switch_control -- 192.168.1.100:6053
//! ```

use esphome_client::{
    types::{
        EspHomeMessage, ListEntitiesRequest, ListEntitiesSwitchResponse, SwitchCommandRequest,
    },
    EspHomeClient,
};
use std::collections::HashMap;
use std::io::Write;

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

    // Discover switches
    let mut switches: HashMap<u32, ListEntitiesSwitchResponse> = HashMap::new();

    println!("\nDiscovering switches...");

    // Request list of all entities
    client.try_write(ListEntitiesRequest {}).await?;

    // Read initial messages to discover all switches
    let mut discovery_complete = false;
    while !discovery_complete {
        let response = client.try_read().await?;
        match response {
            EspHomeMessage::ListEntitiesSwitchResponse(switch) => {
                println!("[{}] {} - {}", switch.key, switch.name, switch.object_id);
                switches.insert(switch.key, switch);
            }
            EspHomeMessage::ListEntitiesDoneResponse(_) => {
                discovery_complete = true;
            }
            _ => {}
        }
    }

    if switches.is_empty() {
        println!("\nNo switches found on this device.");
        return Ok(());
    }

    // Interactive control loop
    println!("\n=== Switch Control ===");
    loop {
        println!("\nAvailable switches:");
        for (key, switch) in &switches {
            println!("  {} - {}", key, switch.name);
        }
        println!("\nEnter command (format: <key> <on|off>) or 'quit' to exit:");
        print!("> ");
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input == "quit" {
            break;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() != 2 {
            println!("Invalid format. Use: <key> <on|off>");
            continue;
        }

        let key: u32 = match parts[0].parse() {
            Ok(k) => k,
            Err(_) => {
                println!("Invalid key. Must be a number.");
                continue;
            }
        };

        let state = match parts[1].to_lowercase().as_str() {
            "on" | "true" | "1" => true,
            "off" | "false" | "0" => false,
            _ => {
                println!("Invalid state. Use 'on' or 'off'.");
                continue;
            }
        };

        if !switches.contains_key(&key) {
            println!("Switch with key {} not found.", key);
            continue;
        }

        #[allow(unused_variables)]
        let switch = &switches[&key];

        // Send switch command
        client
            .try_write(SwitchCommandRequest {
                key,
                state,
                #[cfg(not(any(feature = "api-1-10", feature = "api-1-9", feature = "api-1-8")))]
                device_id: switch.device_id,
            })
            .await?;

        println!(
            "Sent command to turn {} switch '{}'",
            if state { "ON" } else { "OFF" },
            switches[&key].name
        );
    }

    // Close connection
    client.close().await?;
    println!("\nConnection closed.");

    Ok(())
}
