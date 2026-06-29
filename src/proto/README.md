# Api versions

Proto files are sourced from: https://github.com/esphome/esphome/blob/main/esphome/components/api/. 

Unfortunately it's quite tricky to find which api version belongs to which esphome version. Best is 
to look for the api version response in the [src code](https://github.com/esphome/esphome/blob/f5aab154a6182eb3963bebcf47b777eec4953c60/esphome/components/api/api_connection.cpp#L1556).

All files in this directory are either fetched from the esphome repository or generated using the proto builder crate. The proto builder crate is used to generate Rust code from the proto files. See [esphome-client-proto-builder/README.md](../esphome-client-proto-builder/README.md) for more information.