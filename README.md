[![continuous integration](https://github.com/daanoz/esphome-client/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/daanoz/esphome-client/actions/workflows/ci.yml?query=branch%3Amaster)
[![Documentation](https://docs.rs/esphome-client/badge.svg)](https://docs.rs/esphome-client/)
[![Crate](https://img.shields.io/crates/v/esphome-client.svg)](https://crates.io/crates/esphome-client)
[![Dependency Status](https://deps.rs/repo/github/daanoz/esphome-client/status.svg)](https://deps.rs/repo/github/daanoz/esphome-client)

# ESPHome Client

This crate contains a library for interacting with ESPHome devices using sockets. It provides all the necessary functionality to connect, authenticate, and communicate with ESPHome devices. The library is designed to be used as a dependency in other Rust projects that require communication with ESPHome devices.

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

Currently supported (the newest version is the default):
<!-- API_VERSIONS -->
- 1.16 (`api-1-16`) [(2026.9.0b1)](https://github.com/esphome/esphome/blob/2026.9.0b1/esphome/components/api/api.proto)
- 1.15 (`api-1-15`) [(2026.8.0b2)](https://github.com/esphome/esphome/blob/2026.8.0b2/esphome/components/api/api.proto)
- 1.14 (`api-1-14`) [(2026.1.0)](https://github.com/esphome/esphome/blob/2026.1.0/esphome/components/api/api.proto)
- 1.13 (`api-1-13`) [(2025.11.0)](https://github.com/esphome/esphome/blob/2025.11.0/esphome/components/api/api.proto)
- 1.12 (`api-1-12`) [(2025.8.0)](https://github.com/esphome/esphome/blob/2025.8.0/esphome/components/api/api.proto)
- 1.10 (`api-1-10`) [(2025.5.0)](https://github.com/esphome/esphome/blob/2025.5.0/esphome/components/api/api.proto)
- 1.9 (`api-1-9`) [(2024.4.0)](https://github.com/esphome/esphome/blob/2024.4.0/esphome/components/api/api.proto)
- 1.8 (`api-1-8`) [(2023.5.0)](https://github.com/esphome/esphome/blob/2023.5.0/esphome/components/api/api.proto)

Follow [the guide](src/proto/README.md) in the proto dir to see how to add a new version.

After automatic setup, `client.connection_info()` exposes the peer's advertised
API version, software identification and node name. Manual setup
(`without_connection_setup()`) leaves this metadata as `None` and does not
consume a `HelloResponse`. A different API major is rejected; a different minor
produces a warning and requires the application to select compatible operations.
This does not guarantee compatibility with every future firmware release.

The builder's `timeout()` covers the complete TCP, Noise, Hello and optional
authentication setup with one deadline. It does not impose an idle read timeout.

### Transport and wire errors

Network writes use fallible encoding and return an error for oversized messages
before changing the Noise nonce. The protobuf payload limits are 65,535 bytes for
plaintext and 65,515 bytes for Noise (four bytes of internal header plus a
16-byte authentication tag). Rejected oversized messages leave the session usable.
Cloned write streams serialize encoding and complete writes. A write error or
cancellation after encoding starts terminates the whole session; reconnect before
using it again. Closing or dropping the client also invalidates its write clones.

`EspHomeMessage::decode_wire()` distinguishes unknown message IDs, invalid lengths
and malformed known messages through `MessageDecodeError`. An unknown ID does not
consume the next frame. Malformed known messages terminate the session.
`encode_wire()` returns size errors rather than panicking. The legacy
`From<EspHomeMessage> for Vec<u8>` remains infallible and can still panic for an
oversized message; network methods do not use it. The existing `TryFrom<Vec<u8>>`
continues to return `String` errors.

Adding error variants affects downstream exhaustive matches on the public error
enums. The broader setup timeout is also an observable behavior change.

## Future

Some things to be added/improved in the future:

- Connection pooling

## License

`esphome-client` is distributed under the terms of the MIT License.

See [LICENSE](https://github.com/daanoz/esphome-client/blob/main/LICENSE) for details.

Copyright 2025 Daan Sieben
