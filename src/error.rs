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
    Authentication { reason: String },

    /// Stream-related errors.
    #[error("Stream error: {0}")]
    Stream(#[from] StreamError),

    /// Protocol parsing errors.
    #[error("Protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    /// Timeout during operation.
    #[error("Operation timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u128 },

    /// Configuration error.
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Protocol mismatch whilst connecting.
    #[error("Protocol mismatch: expected {expected}, actual {actual}")]
    ProtocolMismatch { expected: String, actual: String },

    /// Invalid internal state.
    #[error("Invalid internal state: {reason}")]
    InvalidInternalState { reason: String },
}

/// Connection-specific errors.
#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    /// Failed to establish TCP connection.
    #[error("Failed to connect to {address}: {source}")]
    TcpConnect {
        address: String,
        #[source]
        source: StdIoError,
    },

    /// Noise protocol handshake failed.
    #[error("Noise handshake failed: {reason}")]
    NoiseHandshake { reason: String },
}

/// Stream-related errors.
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    /// Invalid frame format received.
    #[error("Invalid frame format: {reason}")]
    InvalidFrame { reason: String },

    /// Frame size exceeds maximum allowed size.
    #[error("Frame too large: {size} bytes (max: {max_size})")]
    FrameTooLarge { size: usize, max_size: usize },

    /// Failed to read from stream.
    #[error("Read error: {source}")]
    Read {
        #[source]
        source: StdIoError,
    },

    /// Failed to write to stream.
    #[error("Write error: {source}")]
    Write {
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
        #[source]
        source: prost::DecodeError,
    },

    /// Failed to encode protobuf message.
    #[error("Protobuf encoding failed: {source}")]
    ProtobufEncode {
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
    ValidationFailed { reason: String },
}

/// Discovery-related errors.
#[derive(Debug, thiserror::Error)]
pub enum DiscoveryError {
    /// Error during initialization of the discovery client.
    #[error("Initialization error: {reason}")]
    InitializationError { reason: String },

    /// Discovery was aborted, e.g., due to a shutdown signal.
    #[error("Discovery aborted")]
    Aborted,
}

/// Noise protocol specific errors.
#[derive(Debug, thiserror::Error)]
pub enum NoiseError {
    /// Noise handshake state error.
    #[error("Noise handshake error: {reason}")]
    Handshake { reason: String },

    /// Noise transport state error.
    #[error("Noise transport error: {reason}")]
    Transport { reason: String },

    /// Invalid noise key format.
    #[error("Invalid noise key: {reason}")]
    InvalidKey { reason: String },

    /// Noise encryption/decryption failed.
    #[error("Noise crypto operation failed: {reason}")]
    CryptoOperation { reason: String },
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
