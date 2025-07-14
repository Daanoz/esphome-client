//! List all entities example
//!
//! This example demonstrates how to list all entities (sensors, switches, lights, etc.)
//! available on an ESPHome device.
//!
//! Usage:
//! ```bash
//! cargo run --example list_all_entities -- <host:port> [api_key]
//! # Example: cargo run --example list_all_entities -- 192.168.1.100:6053
//! ```

use esphome_client::{
    types::{EspHomeMessage, ListEntitiesRequest},
    EspHomeClient,
};

#[allow(unused_mut, reason = "support multiple versions")]
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

    println!("Connected successfully!\n");

    // Request list of all entities
    client.try_write(ListEntitiesRequest {}).await?;

    // Counters for each entity type
    let mut sensors = 0;
    let mut binary_sensors = 0;
    let mut switches = 0;
    let mut lights = 0;
    let mut fans = 0;
    let mut covers = 0;
    let mut numbers = 0;
    let mut selects = 0;
    let mut buttons = 0;
    let mut texts = 0;

    // Read and display all entities
    loop {
        let response = client.try_read().await?;
        match response {
            EspHomeMessage::ListEntitiesSensorResponse(entity) => {
                sensors += 1;
                println!(
                    "[Sensor] {} - {} {}",
                    entity.name, entity.device_class, entity.unit_of_measurement
                );
            }
            EspHomeMessage::ListEntitiesBinarySensorResponse(entity) => {
                binary_sensors += 1;
                println!("[Binary Sensor] {} - {}", entity.name, entity.device_class);
            }
            EspHomeMessage::ListEntitiesSwitchResponse(entity) => {
                switches += 1;
                println!(
                    "[Switch] {} - {}",
                    entity.name,
                    if entity.assumed_state {
                        "assumed state"
                    } else {
                        "known state"
                    }
                );
            }
            EspHomeMessage::ListEntitiesLightResponse(entity) => {
                lights += 1;
                println!("[Light] {}", entity.name);
            }
            EspHomeMessage::ListEntitiesFanResponse(entity) => {
                fans += 1;
                println!("[Fan] {}", entity.name);
            }
            EspHomeMessage::ListEntitiesCoverResponse(entity) => {
                covers += 1;
                println!("[Cover] {} - {}", entity.name, entity.device_class);
            }
            EspHomeMessage::ListEntitiesNumberResponse(entity) => {
                numbers += 1;
                println!(
                    "[Number] {} - {} to {} {}",
                    entity.name, entity.min_value, entity.max_value, entity.unit_of_measurement
                );
            }
            EspHomeMessage::ListEntitiesSelectResponse(entity) => {
                selects += 1;
                println!(
                    "[Select] {} - {} options",
                    entity.name,
                    entity.options.len()
                );
            }
            EspHomeMessage::ListEntitiesButtonResponse(entity) => {
                buttons += 1;
                println!("[Button] {} - {}", entity.name, entity.device_class);
            }
            #[cfg(not(any(feature = "api-1-8",)))]
            EspHomeMessage::ListEntitiesTextResponse(entity) => {
                texts += 1;
                println!("[Text] {} - max {} chars", entity.name, entity.max_length);
            }
            EspHomeMessage::ListEntitiesDoneResponse(_) => {
                break;
            }
            _ => {
                // Ignore other message types during entity listing
            }
        }
    }

    // Print summary
    println!("\n=== Summary ===");
    println!("Sensors: {}", sensors);
    println!("Binary Sensors: {}", binary_sensors);
    println!("Switches: {}", switches);
    println!("Lights: {}", lights);
    println!("Fans: {}", fans);
    println!("Covers: {}", covers);
    println!("Numbers: {}", numbers);
    println!("Selects: {}", selects);
    println!("Buttons: {}", buttons);
    println!("Texts: {}", texts);
    println!(
        "\nTotal entities: {}",
        sensors
            + binary_sensors
            + switches
            + lights
            + fans
            + covers
            + numbers
            + selects
            + buttons
            + texts
    );

    // Close connection
    client.close().await?;

    Ok(())
}
