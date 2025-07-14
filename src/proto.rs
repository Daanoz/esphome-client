#![allow(clippy::absolute_paths, reason = "Generated prost code")]
#![allow(clippy::must_use_candidate, reason = "Generated prost code")]
#![allow(clippy::doc_markdown, reason = "Generated prost code")]
#![allow(clippy::missing_const_for_fn, reason = "Generated prost code")]
#![allow(clippy::struct_excessive_bools, reason = "Generated prost code")]
#![allow(clippy::derive_partial_eq_without_eq, reason = "Generated prost code")]
#![allow(clippy::empty_structs_with_brackets, reason = "Generated prost code")]
#[cfg(any(
    all(
        feature = "api-1-12",
        any(feature = "api-1-10", feature = "api-1-9", feature = "api-1-8")
    ),
    all(
        feature = "api-1-10",
        any(feature = "api-1-12", feature = "api-1-9", feature = "api-1-8")
    ),
    all(
        feature = "api-1-9",
        any(feature = "api-1-12", feature = "api-1-10", feature = "api-1-8")
    ),
    all(
        feature = "api-1-8",
        any(feature = "api-1-12", feature = "api-1-10", feature = "api-1-9")
    ),
))]
compile_error!("Cannot combine multiple API version features. Please enable only one of them.");
#[cfg(not(any(
    feature = "api-1-12",
    feature = "api-1-10",
    feature = "api-1-9",
    feature = "api-1-8"
)))]
include!(concat!(env!("OUT_DIR"), "/esphome_proto_1.12.rs")); // Default to latest
#[cfg(feature = "api-1-12")]
include!(concat!(env!("OUT_DIR"), "/esphome_proto_1.12.rs"));
#[cfg(feature = "api-1-10")]
include!(concat!(env!("OUT_DIR"), "/esphome_proto_1.10.rs"));
#[cfg(feature = "api-1-9")]
include!(concat!(env!("OUT_DIR"), "/esphome_proto_1.9.rs"));
#[cfg(feature = "api-1-8")]
include!(concat!(env!("OUT_DIR"), "/esphome_proto_1.8.rs"));
