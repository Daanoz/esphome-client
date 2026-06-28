use heck::ToUpperCamelCase;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use std::path::Path;
use regex::Regex;

fn main() {
    let manifest_path = env!("CARGO_MANIFEST_DIR");
    let repo_root = Path::new(manifest_path).parent().expect("Failed to get parent directory of manifest path");
    let proto_dir = repo_root.join("src/proto");

    println!("Generating Rust code from proto files in {:?}", proto_dir);

    let mut versions = vec![];
    for entry in std::fs::read_dir(&proto_dir).expect("Failed to read proto directory") {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.is_dir() {
            let version = path.file_name().expect("Failed to get directory name").to_str().expect("Failed to convert OsStr to str");
            versions.push(version.to_string());
            let version = version.trim_start_matches("api_").replace('_', ".");
            println!("Generating code for ESPHome API version {}", version);
            generate_code_for_version(&version, &path);
        }
    }

    generate_proto_api_file(&proto_dir, versions);
}

// Generates Rust code for a specific ESPHome API version from the proto files in the given path.
fn generate_code_for_version(version: &str, path: &Path) {
    let proto_file = path.join("api.proto").to_string_lossy().to_string();
    let service_generator = Box::new(ServiceGenerator::new(version, &proto_file));
    let mut config = prost_build::Config::new();
    config.default_package_filename("mod");
    config.service_generator(service_generator);
    config.out_dir(path);
    config.compile_protos(&[&proto_file], &[path]).unwrap();
}

// Generates the `api.rs` file that includes the correct module based on the enabled feature.
fn generate_proto_api_file(path: &Path, mut versions: Vec<String>) {
    let api_file_path = path.join("api.rs");
    let mut content = String::from(
        "// This file is generated automatically. Do not edit manually.\n\n"
    );
    versions.sort_by(|a, b| 
    {
        let a_parts: Vec<u32> = a.trim_start_matches("api_").split('_').map(|s| s.parse::<u32>().unwrap()).collect();
        let b_parts: Vec<u32> = b.trim_start_matches("api_").split('_').map(|s| s.parse::<u32>().unwrap()).collect();
        a_parts.cmp(&b_parts)
    });
    versions.reverse(); // Sort in descending order to have the latest version first
    let default_version = versions.first().expect("No versions found");

    // Mutually exclusive feature checks
    content.push_str("// Ensure that only one of the specified features can be enabled at a time\n#[cfg(any(\n");
    for version in &versions {
        let version_feature = version_to_feature_name(version);
        content.push_str(&format!("    all(feature = \"{version_feature}\", any({})),\n", list_other_features(&versions, version)));
    }
    content.push_str("))]\ncompile_error!(\"Cannot combine multiple API version features. Please enable only one of them.\");\n");

    // Include module matching feature flags for each version
    for version in &versions {
        let version_feature = version_to_feature_name(version);
        let other_versions = list_other_features(&versions, version);

        // Add default case
        if version == default_version {
            content.push_str(&format!("
// If no feature is specified, default to the latest version ({version})
#[cfg(not(any(feature = \"{version_feature}\", {other_versions})))]
mod {version};
#[cfg(not(any(feature = \"{version_feature}\", {other_versions})))]
pub use {version}::*;
"));
        }
        content.push_str(&format!("
// If feature \"{version_feature}\" is specified, include the corresponding module
#[cfg(all(feature = \"{version_feature}\", not(any({other_versions}))))]
mod {version};
#[cfg(all(feature = \"{version_feature}\", not(any({other_versions}))))]
pub use {version}::*;
"));
    }

    std::fs::write(api_file_path, content).expect("Failed to write api.rs file");
}

fn list_other_features(versions: &[String], current_version: &str) -> String {
    versions.iter()
        .filter(|&v| v != current_version)
        .map(|v| format!("feature = \"{}\"", version_to_feature_name(v)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn version_to_feature_name(version: &str) -> String {
    version.replace('_', "-")
}

struct ServiceGenerator {
    version: String,
    types: Vec<(Ident, u16)>,
}

impl ServiceGenerator {
    pub fn new(version: &str, proto_file: &str) -> Self {
        // Prost is not able to extract custom MessageOptions, so we need to do that ourselves to get to the message id.
        let content = std::fs::read_to_string(proto_file).expect("Failed to read proto file");
        let re = Regex::new(r"message ([A-Za-z]+) \{[^}]*option ?\(id\) ?= ?([\d]+);").unwrap();

        let types = re
            .captures_iter(&content)
            .map(|m| {
                let message_name = m.get(1).unwrap().as_str().to_string();
                let message_id: u16 = m
                    .get(2)
                    .unwrap()
                    .as_str()
                    .parse()
                    .expect("Failed to parse message id");
                (
                    format_ident!("{}", message_name.to_upper_camel_case()),
                    message_id,
                )
            })
            .collect();

        ServiceGenerator {
            version: version.to_string(),
            types,
        }
    }
}

impl prost_build::ServiceGenerator for ServiceGenerator {
    fn generate(&mut self, _service: prost_build::Service, out: &mut String) {
        let (major, minor) = self
            .version
            .split_once('.')
            .expect("Version should be in format X.Y");
        let major: u32 = major.parse().expect("Major version should be a number");
        let minor: u32 = minor.parse().expect("Minor version should be a number");
        let enum_name = format_ident!("EspHomeMessage");
        let variants = self
            .types
            .iter()
            .map(|(message_name, _)| message_name)
            .collect::<Vec<_>>();
        let variant_to_typeid = self
            .types
            .iter()
            .map(|(message_name, message_id)| quote! { #message_name(_) => #message_id })
            .collect::<Vec<_>>();
        let typeid_to_variant = self
            .types
            .iter()
            .map(|(message_name, message_id)| quote! { #message_id => #message_name::decode(payload).map(#enum_name::#message_name) })
            .collect::<Vec<_>>();
        out.push_str(
            quote! {
                pub const API_VERSION: (u32, u32) = (#major, #minor);

                #[derive(Clone, Debug, PartialEq)]
                pub enum #enum_name {
                   #(#variants(#variants)),*
                }
                impl #enum_name {
                    #[allow(clippy::too_many_lines, reason = "Generated code for all messages")]
                    const fn get_message_type(&self) -> u16 {
                        match self {
                            #(Self::#variant_to_typeid,)*
                        }
                    }
                }
                impl From<#enum_name> for Vec<u8> {
                    #[allow(clippy::too_many_lines, reason = "Generated code for all messages")]
                    fn from(val: #enum_name) -> Self {
                        use prost::Message as _;

                        let type_id = val.get_message_type();
                        let payload = match val {
                            #(#enum_name::#variants(d) => d.encode_to_vec(),)*
                        };
                        let payload_len = u16::try_from(payload.len()).expect("Payload length exceeds u16::MAX");
                        [
                            type_id.to_be_bytes().to_vec(),
                            payload_len.to_be_bytes().to_vec(),
                            payload
                        ].concat()
                    }
                }
                impl TryFrom<Vec<u8>> for #enum_name {
                    type Error = String;
                    #[allow(clippy::too_many_lines, reason = "Generated code for all messages")]
                    fn try_from(msg: Vec<u8>) -> Result<Self, Self::Error> {
                        use prost::Message as _;
                        if msg.len() < 4 {
                            return Err("Message too short".to_owned());
                        }
                        let type_id = u16::from_be_bytes([msg[0], msg[1]]);
                        // let size = u16::from_be_bytes([msg[2], msg[3]]);
                        let payload = &msg[4..];
                        match type_id {
                            #(#typeid_to_variant,)*
                            _ => return Err(format!("Unknown message type: {type_id}")),
                        }.map_err(|e| format!("Failed to decode message: {e}"))
                    }
                }
            }
            .to_string()
            .as_str(),
        );

        let conversions = self
            .types
            .iter()
            .map(|(message_name, _)| {
                quote! {
                    impl From<#message_name> for #enum_name {
                        fn from(msg: #message_name) -> Self {
                            Self::#message_name(msg)
                        }
                    }
                }
            })
            .collect::<Vec<_>>();
        out.push_str(
            quote! {
                #(#conversions)*
            }
            .to_string()
            .as_str(),
        );
    }
}
