#![allow(
    clippy::module_name_repetitions,
    reason = "Error suffix is for readability"
)]
use std::io::Error as StdIoError;

/// Main error type for ESPHome client operations.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// Connection-related errors.
    #[error("Connection error: {0}")]
    Connection(#[from] ConnectionError),

    /// Authentication failed during handshake.
    #[error("Authentication failed: {reason}")]
    Authentication {
        /// Reason why authentication has failed.
        reason: String,
    },

    /// Stream-related errors.
    #[error("Stream error: {0}")]
    Stream(#[from] StreamError),

    /// Protocol parsing errors.
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Timeout during operation.
    #[error("Operation timed out after {timeout_ms}ms")]
    Timeout {
        /// Duration in milliseconds after which the operation timed out.
        timeout_ms: u128,
    },

    /// Configuration error.
    #[error("Configuration error: {message}")]
    Configuration {
        /// Description of the configuration error.
        message: String,
    },

    /// Protocol mismatch whilst connecting.
    #[error("Protocol mismatch: expected {expected}, actual {actual}")]
    ProtocolMismatch {
        /// Expected protocol version.
        expected: String,
        /// Actual protocol version.
        actual: String,
    },

    /// Invalid internal state.
    #[error("Invalid internal state: {reason}")]
    InvalidInternalState {
        /// Reason for the invalid internal state.
        reason: String,
    },
}

/// Connection-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    /// Failed to establish TCP connection.
    #[error("Failed to connect to {address}: {source}")]
    TcpConnect {
        /// Address we attempted to connect to.
        address: String,
        /// Source IO error.
        #[source]
        source: StdIoError,
    },

    /// Noise protocol handshake failed.
    #[error("Noise handshake failed: {reason}")]
    NoiseHandshake {
        /// Reason for the handshake failure.
        reason: String,
    },
}

/// Stream-related errors.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// Invalid frame format received.
    #[error("Invalid frame format: {reason}")]
    InvalidFrame {
        /// Reason why the frame is invalid.
        reason: String,
    },

    /// Frame size exceeds maximum allowed size.
    #[error("Frame too large: {size} bytes (max: {max_size})")]
    FrameTooLarge {
        /// Size of the frame.
        size: usize,
        /// Maximum allowed size.
        max_size: usize,
    },

    /// Failed to read from stream.
    #[error("Read error: {source}")]
    Read {
        /// Source IO error.
        #[source]
        source: StdIoError,
    },

    /// Failed to write to stream.
    #[error("Write error: {source}")]
    Write {
        /// Source IO error.
        #[source]
        source: StdIoError,
    },
}

/// Protocol-related errors.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    /// Failed to parse protobuf message.
    #[error("Protobuf parsing failed: {source}")]
    ProtobufParse {
        /// Source decode error.
        #[source]
        source: prost::DecodeError,
    },

    /// Failed to encode protobuf message.
    #[error("Protobuf encoding failed: {source}")]
    ProtobufEncode {
        /// Source encode error.
        #[source]
        source: prost::EncodeError,
    },

    /// Unexpected encryption received.
    #[error("Unexpected plain data: Device is notusing noise encryption protocol")]
    UnexpectedPlain,

    /// Unexpected encryption received.
    #[error("Unexpected encryption: Device is using noise encryption protocol")]
    UnexpectedEncryption,

    /// Message validation failed.
    #[error("Message validation failed: {reason}")]
    ValidationFailed {
        /// Reason for validation failure.
        reason: String,
    },
}

/// Discovery-related errors.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// Error during initialization of the discovery client.
    #[error("Initialization error: {reason}")]
    InitializationError {
        /// Reason for the initialization error.
        reason: String,
    },

    /// Discovery was aborted, e.g., due to a shutdown signal.
    #[error("Discovery aborted")]
    Aborted,
}

/// Noise protocol specific errors.
#[derive(Debug, thiserror::Error)]
pub enum NoiseError {
    /// Noise handshake state error.
    #[error("Noise handshake error: {reason}")]
    Handshake {
        /// Reason for the handshake error.
        reason: String,
    },

    /// Noise transport state error.
    #[error("Noise transport error: {reason}")]
    Transport {
        /// Reason for the transport error.
        reason: String,
    },

    /// Invalid noise key format.
    #[error("Invalid noise key: {reason}")]
    InvalidKey {
        /// Reason for the invalid key error.
        reason: String,
    },

    /// Noise encryption/decryption failed.
    #[error("Noise crypto operation failed: {reason}")]
    CryptoOperation {
        /// Reason for the crypto operation error.
        reason: String,
    },
}

/// Convert snow errors to `NoiseError`.
impl From<snow::Error> for NoiseError {
    fn from(err: snow::Error) -> Self {
        match err {
            snow::Error::Init(_) => Self::Handshake {
                reason: err.to_string(),
            },
            snow::Error::Decrypt => Self::CryptoOperation {
                reason: "Decryption failed".to_owned(),
            },
            _ => Self::Transport {
                reason: err.to_string(),
            },
        }
    }
}

/// Convert `NoiseError` to `ClientError`.
impl From<NoiseError> for ClientError {
    fn from(err: NoiseError) -> Self {
        Self::Connection(ConnectionError::NoiseHandshake {
            reason: err.to_string(),
        })
    }
}

/// Convert `prost` errors to `ProtocolError`.
impl From<prost::DecodeError> for ProtocolError {
    fn from(err: prost::DecodeError) -> Self {
        Self::ProtobufParse { source: err }
    }
}

impl From<prost::EncodeError> for ProtocolError {
    fn from(err: prost::EncodeError) -> Self {
        Self::ProtobufEncode { source: err }
    }
}
