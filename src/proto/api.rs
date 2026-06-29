// This file is generated automatically. Do not edit manually.

// Ensure that only one of the specified features can be enabled at a time
#[cfg(any(
    all(feature = "api-1-14", any(feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8")),
    all(feature = "api-1-13", any(feature = "api-1-14", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8")),
    all(feature = "api-1-12", any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8")),
    all(feature = "api-1-10", any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-9", feature = "api-1-8")),
    all(feature = "api-1-9", any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-8")),
    all(feature = "api-1-8", any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9")),
))]
compile_error!("Cannot combine multiple API version features. Please enable only one of them.");

// If no feature is specified, default to the latest version (api_1_14)
#[cfg(not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8")))]
mod api_1_14;
#[cfg(not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8")))]
pub use api_1_14::*;

// If feature "api-1-14" is specified, include the corresponding module
#[cfg(all(feature = "api-1-14", not(any(feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8"))))]
mod api_1_14;
#[cfg(all(feature = "api-1-14", not(any(feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8"))))]
pub use api_1_14::*;

// If feature "api-1-13" is specified, include the corresponding module
#[cfg(all(feature = "api-1-13", not(any(feature = "api-1-14", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8"))))]
mod api_1_13;
#[cfg(all(feature = "api-1-13", not(any(feature = "api-1-14", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8"))))]
pub use api_1_13::*;

// If feature "api-1-12" is specified, include the corresponding module
#[cfg(all(feature = "api-1-12", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8"))))]
mod api_1_12;
#[cfg(all(feature = "api-1-12", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-10", feature = "api-1-9", feature = "api-1-8"))))]
pub use api_1_12::*;

// If feature "api-1-10" is specified, include the corresponding module
#[cfg(all(feature = "api-1-10", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-9", feature = "api-1-8"))))]
mod api_1_10;
#[cfg(all(feature = "api-1-10", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-9", feature = "api-1-8"))))]
pub use api_1_10::*;

// If feature "api-1-9" is specified, include the corresponding module
#[cfg(all(feature = "api-1-9", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-8"))))]
mod api_1_9;
#[cfg(all(feature = "api-1-9", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-8"))))]
pub use api_1_9::*;

// If feature "api-1-8" is specified, include the corresponding module
#[cfg(all(feature = "api-1-8", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9"))))]
mod api_1_8;
#[cfg(all(feature = "api-1-8", not(any(feature = "api-1-14", feature = "api-1-13", feature = "api-1-12", feature = "api-1-10", feature = "api-1-9"))))]
pub use api_1_8::*;
