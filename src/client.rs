/// Represents a TCP stream connection to an ESPHome API server.
///
/// This struct manages the underlying stream reader and writer, and provides methods for sending and receiving
/// ESPHome protocol messages. It can optionally handle ping requests automatically to keep the connection alive.
///
/// Use [`EspHomeTcpStream::builder`] to create a builder for establishing a connection.
mod noise;
mod plain;

mod stream_reader;
mod stream_writer;
use std::{fmt::Debug, time::Duration};

use stream_reader::StreamReader;
use stream_writer::StreamWriter;
use tokio::time::timeout;

use crate::{
    error::{ClientError, ProtocolError},
    proto::{ConnectRequest, DisconnectRequest, EspHomeMessage, HelloRequest, PingResponse},
    API_VERSION,
};

type StreamPair = (StreamReader, StreamWriter);

#[derive(Debug)]
pub struct EspHomeClient {
    streams: StreamPair,
    handle_ping: bool,
}

impl EspHomeClient {
    /// Creates a new builder for configuring and connecting to an ESPHome API server.
    #[must_use]
    pub fn builder() -> EspHomeClientBuilder {
        EspHomeClientBuilder::new()
    }

    /// Sends a message to the ESPHome device.
    ///
    /// # Errors
    ///
    /// Will return an error if the write operation fails for example due to a disconnected stream.
    pub async fn try_write<M>(&mut self, message: M) -> Result<(), ClientError>
    where
        M: Into<EspHomeMessage> + Debug,
    {
        tracing::debug!("Send: {message:?}");
        let message: EspHomeMessage = message.into();
        let payload: Vec<u8> = message.into();
        self.streams.1.write_message(payload).await
    }

    /// Reads the next message from the stream.
    ///
    /// It will automatically handle ping requests if ping handling is enabled.
    ///
    /// # Errors
    ///
    /// Will return an error if the read operation fails, for example due to a disconnected stream
    pub async fn try_read(&mut self) -> Result<EspHomeMessage, ClientError> {
        loop {
            let payload = self.streams.0.read_next_message().await?;
            let message: EspHomeMessage =
                payload
                    .clone()
                    .try_into()
                    .map_err(|e| ProtocolError::ValidationFailed {
                        reason: format!("Failed to decode EspHomeMessage: {e}"),
                    })?;
            tracing::debug!("Receive: {message:?}");
            match message {
                EspHomeMessage::PingRequest(_) if self.handle_ping => {
                    self.try_write(PingResponse {}).await?;
                }
                msg => return Ok(msg),
            }
        }
    }

    /// Closes the connection gracefully by sending a `DisconnectRequest` message.
    ///
    /// # Errors
    ///
    /// Will return an error if the write operation fails, for example due to a disconnected stream
    pub async fn close(mut self) -> Result<(), ClientError> {
        self.try_write(DisconnectRequest {}).await?;
        // Dropping self & self.streams will close the streams automatically.
        Ok(())
    }

    /// Returns a clone-able write stream for sending messages to the ESPHome device.
    #[must_use]
    pub fn write_stream(&self) -> EspHomeClientWriteStream {
        EspHomeClientWriteStream {
            writer: self.streams.1.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EspHomeClientWriteStream {
    writer: StreamWriter,
}
impl EspHomeClientWriteStream {
    /// Sends a message to the ESPHome device.
    ///
    /// # Errors
    ///
    /// Will return an error if the write operation fails for example due to a disconnected stream.
    pub async fn try_write<M>(&self, message: M) -> Result<(), ClientError>
    where
        M: Into<EspHomeMessage> + Debug,
    {
        tracing::debug!("Send: {message:?}");
        let message: EspHomeMessage = message.into();
        let payload: Vec<u8> = message.into();
        self.writer.write_message(payload).await
    }
}

#[derive(Debug)]
pub struct EspHomeClientBuilder {
    addr: Option<String>,
    key: Option<String>,
    password: Option<String>,
    client_info: String,
    timeout: Duration,
    connection_setup: bool,
    handle_ping: bool,
}

impl EspHomeClientBuilder {
    fn new() -> Self {
        Self {
            addr: None,
            key: None,
            password: None,
            client_info: format!("{}:{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            timeout: Duration::from_secs(30),
            connection_setup: true,
            handle_ping: true,
        }
    }

    /// Sets the host address of the ESPHome API server to connect to.
    ///
    /// Takes the address of the server in the format "host:port".
    #[must_use]
    pub fn address(mut self, addr: &str) -> Self {
        self.addr = Some(addr.to_owned());
        self
    }

    /// Enables the use of a 32-byte base64-encoded key for encrypted communication.
    ///
    /// If no key is provided, the connection will be established in plain text.
    /// Further reference: <https://esphome.io/components/api.html#configuration-variables>
    #[must_use]
    pub fn key(mut self, key: &str) -> Self {
        self.key = Some(key.to_owned());
        self
    }

    /// Enables the use of a password to authenticate the client.
    ///
    /// Note that this password is deprecated and will be removed in a future version of ESPHome.
    /// This only works if connection setup is enabled.
    #[must_use]
    pub fn password(mut self, password: &str) -> Self {
        self.password = Some(password.to_owned());
        self
    }

    /// Sets the timeout duration during the tcp connection.
    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Sets the client info string that will be sent in the `HelloRequest`.
    ///
    /// Defaults to the package name and version of the client.
    /// This only works if connection setup is enabled.
    #[must_use]
    pub fn client_info(mut self, client_info: &str) -> Self {
        client_info.clone_into(&mut self.client_info);
        self
    }

    /// Disable connection setup messages.
    ///
    /// Most api requests require a connection setup, which requires a sequence of messages to be sent and received.
    /// - `HelloRequest` -> `HelloResponse`
    /// - `ConnectionRequest` -> `ConnectionResponse`
    ///
    /// By disabling this, the connection can be established manually.
    #[must_use]
    pub const fn without_connection_setup(mut self) -> Self {
        self.connection_setup = false;
        self
    }

    /// Disable automatic handling of ping request.
    ///
    /// The ESPHome API server will send a ping request to the client on a regular interval.
    /// The client needs to respond with a `PingResponse` to keep the connection alive.
    #[must_use]
    pub const fn without_ping_handling(mut self) -> Self {
        self.handle_ping = false;
        self
    }

    /// Connect to the ESPHome API server.
    ///
    /// # Errors
    ///
    /// Will return an error if the connection fails, or if the connection setup fails.
    pub async fn connect(self) -> Result<EspHomeClient, ClientError> {
        let addr = self.addr.ok_or_else(|| ClientError::Configuration {
            message: "Address is not set".into(),
        })?;

        let streams = timeout(self.timeout, async {
            match self.key {
                Some(key) => noise::connect(&addr, &key).await,
                None => plain::connect(&addr).await,
            }
        })
        .await
        .map_err(|_e| ClientError::Timeout {
            timeout_ms: self.timeout.as_millis(),
        })??;

        let mut stream = EspHomeClient {
            streams,
            handle_ping: self.handle_ping,
        };
        if self.connection_setup {
            Self::connection_setup(&mut stream, self.client_info, self.password).await?;
        }
        Ok(stream)
    }

    /// Sets up the connection by sending the `HelloRequest` and `ConnectRequest` messages.
    ///
    /// Details: <https://github.com/esphome/aioesphomeapi/blob/4707c424e5dab921fa15466ecc31148a8c0ee4a9/aioesphomeapi/api.proto#L85>
    async fn connection_setup(
        stream: &mut EspHomeClient,
        client_info: String,
        password: Option<String>,
    ) -> Result<(), ClientError> {
        stream
            .try_write(HelloRequest {
                client_info,
                api_version_major: API_VERSION.0,
                api_version_minor: API_VERSION.1,
            })
            .await?;
        loop {
            let response = stream.try_read().await?;
            match response {
                EspHomeMessage::HelloResponse(response) => {
                    if response.api_version_major != API_VERSION.0 {
                        return Err(ClientError::ProtocolMismatch {
                            expected: format!("{}.{}", API_VERSION.0, API_VERSION.1),
                            actual: format!(
                                "{}.{}",
                                response.api_version_major, response.api_version_minor
                            ),
                        });
                    }
                    if response.api_version_minor != API_VERSION.1 {
                        tracing::warn!(
                            "API version mismatch: expected {}.{}, got {}.{}, expect breaking changes in messages",
                            API_VERSION.0,
                            API_VERSION.1,
                            response.api_version_major,
                            response.api_version_minor
                        );
                    }
                    break;
                }
                _ => {
                    tracing::debug!("Unexpected response during connection setup: {response:?}");
                }
            }
        }
        stream
            .try_write(ConnectRequest {
                password: password.unwrap_or_default(),
            })
            .await?;
        loop {
            let response = stream.try_read().await?;
            match response {
                EspHomeMessage::ConnectResponse(response) => {
                    if response.invalid_password {
                        return Err(ClientError::Authentication {
                            reason: "Invalid password".to_owned(),
                        });
                    }
                    tracing::info!("Connection to ESPHome API established successfully.");
                    break;
                }
                _ => {
                    tracing::debug!("Unexpected response during connection setup: {response:?}");
                }
            }
        }
        Ok(())
    }
}
