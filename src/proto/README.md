# Api versions

Proto files are sourced from: https://github.com/esphome/esphome/blob/main/esphome/components/api/. 

Unfortunately it's quite tricky to find which api version belongs to which esphome version. Best is 
to look for the api version response in the [src code](https://github.com/esphome/esphome/blob/f5aab154a6182eb3963bebcf47b777eec4953c60/esphome/components/api/api_connection.cpp#L1556).

## Currently supported versions:

- [1.13 (2025.11.0)](https://github.com/esphome/esphome/blob/2025.11.0/esphome/components/api/api.proto)
- [1.12 (2025.8.0)](https://github.com/esphome/esphome/blob/2025.8.0/esphome/components/api/api.proto)
- [1.10 (2025.5.0)](https://github.com/esphome/esphome/blob/2025.5.0/esphome/components/api/api.proto)
- [1.9 (2024.4.0)](https://github.com/esphome/esphome/blob/2024.4.0/esphome/components/api/api.proto)
- [1.8 (2023.5.0)](https://github.com/esphome/esphome/blob/2023.5.0/esphome/components/api/api.proto)

## Adding a version

Adding a new version takes a couple of steps (to be automated, some day...)

- Download proto files into [src/proto/<VERSION_NUMBER>](./)
- Update [Cargo.toml](./../../Cargo.toml) to support new version feature
- Update [build.rs](./../../build.rs) to add the new version to the list of versions
- Update [src/proto.rs](./../proto.rs) to add the correct include matching the feature flag