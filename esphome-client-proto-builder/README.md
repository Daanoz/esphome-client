# Esphome proto builder

This preparer is a tool to automatically generate the code of ESPHome API versions. It takes protobuf files, and generates Rust code for each API version. The preparer is designed to be used in a CI/CD pipeline to ensure that the code is always up-to-date with the latest ESPHome API versions. It eliminates the need for the protobuf compiler to be installed on each system.

To be able to use this crate you will need to install the protobuf compiler `protoc`. Official installation instructions can be found [here](https://github.com/protocolbuffers/protobuf?tab=readme-ov-file#protobuf-compiler-installation). Depending on your system, you can also rely on installers:
- `apt install protobuf-compiler` 
- `brew install protobuf`

## How to use

In general, you won't need to use this crate directly. It is used in the [CI/CD pipeline](../.github/workflows/check-esphome-api.yml) to generate the code for the ESPHome API versions. However, if you want to add a new version, you can use this crate to generate the code for that version.

Create a new directory in src/proto for the new version, and copy the `api.proto` and `api_options.proto` file from the ESPHome repository into that directory. Then run the preparer. The preparer will generate Rust code for the new version and update the `src/proto/api.rs` file to include the new version. Afterwards make sure to add the new version to the `Cargo.toml` file as a feature, and update the README.md file to include the new version.