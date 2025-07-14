use heck::ToUpperCamelCase;
use proc_macro2::Ident;
use quote::{format_ident, quote};
use regex::Regex;
use std::io::Result;

fn main() -> Result<()> {
    let versions = vec!["1.12", "1.10", "1.9", "1.8"];
    for version in versions {
        let dir = format!("src/proto/{}/", version);
        let proto_file = format!("{}api.proto", dir);
        let service_generator = Box::new(ServiceGenerator::new(version, &proto_file));
        let mut config = prost_build::Config::new();
        config.default_package_filename(format!("esphome_proto_{version}"));
        config.service_generator(service_generator);
        config.compile_protos(&[&proto_file], &[dir]).unwrap();
    }
    Ok(())
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
