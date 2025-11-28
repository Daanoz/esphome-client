[![continuous integration](https://github.com/daanoz/esphome-client/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/daanoz/esphome-client/actions/workflows/ci.yml?query=branch%3Amaster)
[![Documentation](https://docs.rs/esphome-client/badge.svg)](https://docs.rs/esphome-client/)
[![Crate](https://img.shields.io/crates/v/esphome-client.svg)](https://crates.io/crates/esphome-client)
[![Dependency Status](https://deps.rs/repo/github/daanoz/esphome-client/status.svg)](https://deps.rs/repo/github/daanoz/esphome-client)

# ESPHome Client

This crate contains a library for interacting with ESPHome devices using sockets. It provides all the necessary functionality to connect, authenticate, and communicate with ESPHome devices. The library is designed to be used as a dependency in other Rust projects that require communication with ESPHome devices.

To be able to compile this crate you will need to install the protobuf compiler `protoc`. Official installation instructions can be found [here](https://github.com/protocolbuffers/protobuf?tab=readme-ov-file#protobuf-compiler-installation). Depending on your system, you can also rely on installers:
- `apt install protobuf-compiler` 
- `brew install protobuf`

## Example

Basic example retrieving device info:

```rust,no_run
use esphome_client::{EspHomeClient, types};

// 32-byte, base64 encoded api key
const KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = EspHomeClient::builder()
        .address("192.168.0.2:6053")
        .key(KEY)
        .connect()
        .await?;

    client.try_write(types::DeviceInfoRequest {}).await?;

    loop {
        let response = client.try_read().await?;
        match response {
            types::EspHomeMessage::DeviceInfoResponse(response) => {
                println!("Received DeviceInfoResponse: {:?}", response);
            }
            _ => {
                println!("Received unsupported message type: {:?}", response);
            }
        }
    }
}
```

## API Versions

Different API versions used during communication can be enabled using features. By default,
it will use the latest available. **Note** this means your implementation might break if 
you don't pin down the used version and this crate is updated. It's recommended that you add the feature flag with the version in your Cargo.toml to avoid unexpected issues in the future.

Currently supported:
- 1.13 (`api-1-13`) (2025.11.0) _this is the current default_
- 1.12 (`api-1-12`) (2025.8.0)
- 1.10 (`api-1-10`) (2025.5.0)
- 1.9 (`api-1-9`) (2024.4.0)
- 1.8 (`api-1-8`) (2023.5.0)

Follow [the guide](src/proto/README.md) in the proto dir to see how to add a new version.

## Future

Some things to be added/improved in the future:

- Automatic updates to newly released ESPHome API versions
- Connection pooling

## License

`esphome-client` is distributed under the terms of the MIT License.

See [LICENSE](https://github.com/daanoz/esphome-client/blob/main/LICENSE) for details.

Copyright 2025 Daan Sieben