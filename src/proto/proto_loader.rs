/// This macro is responsible for loading the appropriate generated proto files based on the enabled features.
macro_rules! include_proto {
    ( $( $list:literal ),* $(,)? ) => {
        mutually_exclusive_features!($($list),*);
        include_proto!(@with_default $($list),*);
    };
    // Include the first version if no version is specified, or if it is the only one active.
    (@with_default $default:literal, $( $other:literal ),* $(,)? ) => {
        #[cfg(not(any(
            feature = $default,
            $(feature = $other),*
        )))]
        include!(concat!(env!("OUT_DIR"), "/esphome_proto_", $default, ".rs")); // Default to latest
        #[cfg(all(feature = $default, not(any($(feature = $other),*))))]
        include!(concat!(env!("OUT_DIR"), "/esphome_proto_", $default, ".rs"));
        include_proto!(@with_version $($other),*; $default);
    };
    // Include next version if it was the only one active
    (@with_version $version:literal, $( $other:literal ),* $(,)? ; $( $other_seen:literal ),* $(,)?) => {
        #[cfg(all(feature = $version, not(any( $(feature = $other),* , $(feature = $other_seen),* ))))]
        include!(concat!(env!("OUT_DIR"), "/esphome_proto_", $version, ".rs"));
        include_proto!(@with_version $($other),*; $version, $($other_seen),*);
    };
    // Include last version if it was the only one active
    (@with_version $version:literal; $( $other_seen:literal ),* $(,)?) => {
        #[cfg(all(feature = $version, not(any($(feature = $other_seen),*))))]
        include!(concat!(env!("OUT_DIR"), "/esphome_proto_", $version, ".rs"));
    };
}

/// This macro ensures that only one of the specified features can be enabled at a time.
macro_rules! mutually_exclusive_features {
    ( $( $list:literal ),* $(,)? ) => {
        mutually_exclusive_features!(@with_version $($list),* ;);
    };
    (@with_version $version:literal, $( $other:literal ),* $(,)? ; $( $other_seen:literal ),* $(,)?) => {
        #[cfg(all(
            feature = $version,
            any($(feature = $other),* , $(feature = $other_seen),*)
        ))]
        compile_error!(concat!("Cannot combine multiple API version features with ", $version, ". Please enable only one of them."));
        mutually_exclusive_features!(@with_version $($other),*; $version, $($other_seen),*);
    };
    (@with_version $version:literal; $( $other_seen:literal ),* $(,)?) => {
        #[cfg(all(
            feature = $version,
            any($(feature = $other_seen),*)
        ))]
        compile_error!(concat!("Cannot combine multiple API version features with ", $version, ". Please enable only one of them."));
    };
}
