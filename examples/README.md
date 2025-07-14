# ESPHome Client Examples

This directory contains practical examples demonstrating how to use the `esphome-client` library.

## Prerequisites

- Rust 1.82.0 or later
- `Protoc` installed 
- An ESPHome device on your local network
- The device's IP address and port (default: 6053)
- API key (if encryption is enabled on the device)

## Running Examples

All examples can be run using `cargo run --example <example_name>`. Most examples require command-line arguments.

You can switch version of the protocol used by using `--features "api-1-12"`. For supported versions, refer to the root [Cargo.toml](../Cargo.toml) file.

### Basic Examples

#### 1. Plain Connection

Connect to an ESPHome device and retrieve device information.

```bash
cargo run --example plain_connection -- 192.168.1.100:6053
```

**What it demonstrates:**
- Establishing a connection
- Sending a request
- Reading responses
- Gracefully closing the connection

---

#### 2. Noise encrypted Connection

Connect using the Noise protocol with API key encryption.

```bash
cargo run --example noise_connection -- 192.168.1.100:6053 YOUR_API_KEY_HERE
```

**What it demonstrates:**
- Encrypted communication using Noise protocol
- API key authentication
- Secure data transmission

**Note:** To find your API key:
1. Check your ESPHome configuration
2. It should be configured under the `api.encryption.key` field

---

#### 3. List All Entities

Discover and list all entities (sensors, switches, lights, etc.) on a device.

```bash
cargo run --example list_all_entities -- 192.168.1.100:6053
# With API key:
cargo run --example list_all_entities -- 192.168.1.100:6053 YOUR_API_KEY
```

**What it demonstrates:**
- Entity discovery
- Handling multiple entity types
- Counting and categorizing entities

---

### Monitoring Examples

#### 4. Sensor Monitoring

Subscribe to sensor updates and monitor values in real-time.

```bash
cargo run --example sensor_monitoring -- 192.168.1.100:6053
```

**What it demonstrates:**
- Subscribing to state updates
- Real-time sensor monitoring
- Handling continuous data streams
- State management

---

### Control Examples

#### 5. Switch Control

Interactively control switches on an ESPHome device.

```bash
cargo run --example switch_control -- 192.168.1.100:6053
```

**What it demonstrates:**
- Switch discovery
- Sending commands
- Interactive user input
- State changes

**Usage in the interactive prompt:**
```
> 12345 on    # Turn switch with key 12345 ON
> 12345 off   # Turn switch with key 12345 OFF
> quit        # Exit the program
```

---

### Discovery Examples

#### 6. Device Discovery

Discover ESPHome devices on your local network using mDNS.

```bash
cargo run --example device_discovery
```

**What it demonstrates:**
- mDNS service discovery
- Finding devices automatically
- Reading device attributes
- Detecting encryption requirements

**Note:** Requires the `discovery` feature to be enabled.

---

## Common Usage Patterns

### Finding Your Device Address

If you don't know your device's IP address, use the device discovery example:

```bash
cargo run --example device_discovery
```

### Getting Device Information

Start with the plain connection example to verify connectivity:

```bash
cargo run --example plain_connection -- YOUR_DEVICE_IP:6053
```

### Working with Encrypted Devices

If your device requires an API key, use the authenticated connection:

```bash
cargo run --example noise_connection -- YOUR_DEVICE_IP:6053 YOUR_API_KEY
```

## Environment Variables

You can set the `RUST_LOG` environment variable to control logging verbosity:

```bash
# Show all debug logs
RUST_LOG=debug cargo run --example sensor_monitoring -- 192.168.1.100:6053

# Show only error logs
RUST_LOG=error cargo run --example sensor_monitoring -- 192.168.1.100:6053
```