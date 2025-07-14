use std::sync::{Arc, Mutex};

use snow::{HandshakeState, TransportState};
use tokio::net::TcpStream;

use crate::error::{ClientError, ConnectionError, NoiseError, ProtocolError, StreamError};

use super::{plain::PLAIN_PREAMBLE, stream_reader::StreamDecoder, stream_writer::StreamEncoder};

use super::{stream_reader::StreamReader, stream_writer::StreamWriter, StreamPair};

const ZERO_BYTE: u8 = 0x00;
const NOISE_PROLOGUE: &[u8; 14] = b"NoiseAPIInit\x00\x00";
const NOISE_HELLO: &[u8; 3] = b"\x01\x00\x00";
pub(super) const NOISE_PREAMBLE: u8 = 0x01;

/// Establishes a TCP connection to the given address and performs a Noise handshake using the provided key.
/// Returns a `StreamPair` with the encrypted streams.
/// For more information on the Noise protocol, see: <http://www.noiseprotocol.org/noise.html#pre-shared-symmetric-keys>
pub(crate) async fn connect(addr: &str, key: &str) -> Result<StreamPair, ClientError> {
    let (read, write) = TcpStream::connect(addr)
        .await
        .map_err(|e| ConnectionError::TcpConnect {
            address: addr.to_owned(),
            source: e,
        })?
        .into_split();
    tracing::debug!("Tcp connection established to {addr}");
    let pre_handshake_decoder: Box<dyn StreamDecoder> = Box::new(PreHandshakeDecoder);
    let (mut reader, writer) = (
        StreamReader::new(read).with_decoder(pre_handshake_decoder),
        StreamWriter::new(write),
    );

    let mut noise_client = create_noise_client(key)?;

    // Handle the Noise handshake
    writer.write_message(noise_hello()).await?;
    writer
        .write_message(noise_handshake(&mut noise_client))
        .await?;
    parse_server_and_mac(reader.read_next_message().await?)?;
    parse_noise_response(reader.read_next_message().await?, &mut noise_client)?;

    // Init coder with noise client
    let coder = NoiseCoder::new(
        noise_client
            .into_transport_mode()
            .map_err(<snow::Error as Into<NoiseError>>::into)?,
    );
    tracing::debug!("Noise handshake completed successfully");
    let decoder: Box<dyn StreamDecoder> = Box::new(coder.clone());
    let encoder: Box<dyn StreamEncoder> = Box::new(coder);
    Ok((reader.with_decoder(decoder), writer.with_encoder(encoder)))
}

// Decoder for pre-handshake frames, which are used to handshake on the encryption protocol.
#[derive(Debug)]
struct PreHandshakeDecoder;
impl StreamDecoder for PreHandshakeDecoder {
    fn decode(&self, buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
        read_frame_from_buffer(buffer)
    }
}

// Decoder and encoder for Noise encrypted frames.
#[derive(Debug, Clone)]
struct NoiseCoder {
    noise: Arc<Mutex<TransportState>>,
}
impl NoiseCoder {
    fn new(noise: TransportState) -> Self {
        Self {
            noise: Arc::new(Mutex::new(noise)),
        }
    }
    fn decrypt(&self, payload: &[u8]) -> Result<Vec<u8>, ClientError> {
        let mut decrypted_payload = vec![0u8; 65535];
        let size = self
            .noise
            .lock()
            .map_err(|e| ClientError::InvalidInternalState {
                reason: format!("Failed to lock noise state: {e}"),
            })?
            .read_message(payload, &mut decrypted_payload)
            .map_err(<snow::Error as Into<NoiseError>>::into)?;
        decrypted_payload.truncate(size);
        Ok(decrypted_payload)
    }
    fn encrypt(&self, payload: &[u8]) -> Result<Vec<u8>, ClientError> {
        let mut encrypted_payload = vec![0u8; 65535];
        let size = self
            .noise
            .lock()
            .map_err(|e| ClientError::InvalidInternalState {
                reason: format!("Failed to lock noise state: {e}"),
            })?
            .write_message(payload, &mut encrypted_payload)
            .map_err(<snow::Error as Into<NoiseError>>::into)?;
        encrypted_payload.truncate(size);
        Ok(encrypted_payload)
    }
}
impl StreamDecoder for NoiseCoder {
    fn decode(&self, buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
        match read_frame_from_buffer(buffer) {
            Ok(Some(data)) => Ok(Some(self.decrypt(&data)?)),
            v => v,
        }
    }
}
impl StreamEncoder for NoiseCoder {
    fn encode(&self, payload: Vec<u8>) -> Result<Vec<u8>, ClientError> {
        let payload = self.encrypt(&payload)?;
        let payload = create_noise_frame(payload);
        Ok(payload)
    }
}

fn create_noise_client(key: &str) -> Result<snow::HandshakeState, ClientError> {
    use base64::{engine::general_purpose, Engine as _};
    let key_bytes = general_purpose::STANDARD
        .decode(key)
        .map_err(|e| NoiseError::InvalidKey {
            reason: e.to_string(),
        })?;
    if key_bytes.len() != 32 {
        return Err(NoiseError::InvalidKey {
            reason: "Invalid PSK length".into(),
        }
        .into());
    }
    let noise = snow::Builder::new(
        "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("Valid encryption protocol"),
    )
    .prologue(NOISE_PROLOGUE)
    .psk(0, &key_bytes)
    .build_initiator()
    .map_err(|e| NoiseError::InvalidKey {
        reason: e.to_string(),
    })?;
    Ok(noise)
}

/// Initial header, indicating a Noise handshake.
fn noise_hello() -> Vec<u8> {
    NOISE_HELLO.to_vec()
}

// Noise handshake message, to verify PSK and establish a secure channel.
fn noise_handshake(noise_client: &mut HandshakeState) -> Vec<u8> {
    let mut payload = vec![0u8; 65535];
    let size = noise_client.write_message(&[], &mut payload).expect("OK");
    payload.truncate(size);
    payload.insert(0, ZERO_BYTE);
    create_noise_frame(payload)
}

// Retrieves the server name and MAC address from the Noise handshake response.
fn parse_server_and_mac(data: Vec<u8>) -> Result<(Option<String>, Option<String>), ClientError> {
    let mut data = data.into_iter();
    if data.next() != Some(NOISE_PREAMBLE) {
        return Err(ProtocolError::UnexpectedPlain.into());
    }
    let mut server_name = None;
    let mut str_bytes = vec![];
    for byte in data.by_ref() {
        if byte == ZERO_BYTE {
            server_name = Some(String::from_utf8_lossy(&str_bytes).to_string());
            tracing::debug!("Server name: {server_name:?}");
            break; // End of the server name
        }
        str_bytes.push(byte);
    }
    let mut mac_address = None;
    str_bytes.clear();
    for byte in data.by_ref() {
        if byte == ZERO_BYTE {
            mac_address = Some(String::from_utf8_lossy(&str_bytes).to_string());
            tracing::debug!("Mac address: {mac_address:?}");
            break; // End of mac address
        }
        str_bytes.push(byte);
    }
    Ok((server_name, mac_address))
}

/// Reads the key verification from noise handshake response
fn parse_noise_response(
    data: Vec<u8>,
    noise_client: &mut HandshakeState,
) -> Result<(), ClientError> {
    let mut data = data.into_iter();
    let preamble = data.next();
    if preamble != Some(ZERO_BYTE) {
        let reason = if data.len() >= 2 {
            String::from_utf8(data.collect()).unwrap_or_else(|_| "Reason not decodable".to_owned())
        } else {
            "Unknown reason".to_owned()
        };
        return Err(ConnectionError::NoiseHandshake {
            reason: format!("Incorrect preamble: {preamble:?}, {reason}"),
        }
        .into());
    }
    let mut handshake_frame = vec![0u8; 65535];
    noise_client
        .read_message(&data.collect::<Vec<u8>>(), &mut handshake_frame)
        .map_err(<snow::Error as Into<NoiseError>>::into)?;
    Ok(())
}

/// Create a frame with the given payload, including the preamble and length.
fn create_noise_frame(payload: Vec<u8>) -> Vec<u8> {
    let frame_len = u16::try_from(payload.len()).expect("Payload length should fit in u16");
    [
        vec![NOISE_PREAMBLE],
        frame_len.to_be_bytes().to_vec(),
        payload,
    ]
    .concat()
}

/// Attempts to read a frame from the buffer.
fn read_frame_from_buffer(buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
    if buffer.len() < 3 {
        return Ok(None);
    }
    let preamble = buffer[0];
    match preamble {
        NOISE_PREAMBLE => {}
        PLAIN_PREAMBLE => {
            return Err(ProtocolError::UnexpectedPlain.into());
        }
        _ => {
            return Err(StreamError::InvalidFrame {
                reason: format!("Invalid preamble: {preamble}"),
            }
            .into());
        }
    }
    let frame_len = usize::from(u16::from_be_bytes([buffer[1], buffer[2]]));
    if buffer.len() < frame_len {
        tracing::debug!(
            "Waiting for more data, expected {} bytes, got {}",
            frame_len,
            buffer.len()
        );
        return Ok(None);
    }
    let frame = buffer.drain(..frame_len + 3).skip(3).collect();
    Ok(Some(frame))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::io;

    fn create_key(seed: u8) -> String {
        use base64::{engine::general_purpose, Engine as _};
        let key = vec![seed; 32];
        general_purpose::STANDARD.encode(key)
    }

    fn create_noise_server(key: &str) -> Result<snow::HandshakeState, io::Error> {
        use base64::{engine::general_purpose, Engine as _};
        let key_bytes = general_purpose::STANDARD
            .decode(key)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if key_bytes.len() != 32 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid PSK length",
            ));
        }
        let noise = snow::Builder::new(
            "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
                .parse()
                .expect("Valid encryption protocol"),
        )
        .prologue(NOISE_PROLOGUE)
        .psk(0, &key_bytes)
        .build_responder()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        Ok(noise)
    }

    #[test]
    fn test_create_noise_frame_and_read_frame_from_buffer() {
        let payload = vec![1, 2, 3, 4, 5];
        let frame = create_noise_frame(payload.clone());
        assert_eq!(frame[0], NOISE_PREAMBLE);
        let len = usize::from(u16::from_be_bytes([frame[1], frame[2]]));
        assert_eq!(len, payload.len());
        let mut buffer = frame;
        let decoded = read_frame_from_buffer(&mut buffer).unwrap();
        assert_eq!(decoded, Some(payload));
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_read_frame_from_buffer_with_insufficient_data() {
        let mut buffer = vec![NOISE_PREAMBLE, 0x00];
        let result = read_frame_from_buffer(&mut buffer);
        assert!(matches!(result, Ok(None)));
    }

    #[test]
    fn test_read_frame_from_buffer_with_unknown_preamble() {
        let mut buffer = vec![0xFF, 0x00, 0x05, 1, 2, 3, 4, 5];
        let result = read_frame_from_buffer(&mut buffer);
        result.unwrap_err();
    }

    #[test]
    fn test_parse_server_and_mac_valid() {
        let mut data = vec![NOISE_PREAMBLE];
        data.extend(b"server_name");
        data.push(ZERO_BYTE);
        data.extend(b"aa:bb:cc:dd:ee:ff");
        data.push(ZERO_BYTE);
        let (server, mac) = parse_server_and_mac(data).unwrap();
        assert_eq!(server, Some("server_name".to_owned()));
        assert_eq!(mac, Some("aa:bb:cc:dd:ee:ff".to_owned()));
    }

    #[test]
    fn test_parse_server_and_mac_invalid_preamble() {
        let mut data = vec![0xFF];
        data.extend(b"server_name");
        data.push(ZERO_BYTE);
        let result = parse_server_and_mac(data);
        result.unwrap_err();
    }

    #[test]
    fn test_noise_hello() {
        let hello = noise_hello();
        assert_eq!(hello, NOISE_HELLO);
    }

    #[test]
    fn test_create_noise_client_invalid_key_length() {
        use base64::{engine::general_purpose, Engine as _};
        let key = general_purpose::STANDARD.encode([0u8; 16]);
        let result = create_noise_client(&key);
        result.unwrap_err();
    }

    #[test]
    fn test_create_noise_client_valid_key() {
        let key = create_key(1u8);
        let result = create_noise_client(&key);
        result.unwrap();
    }

    #[test]
    fn test_noise_handshake_frame_structure() {
        let key = create_key(2u8);
        let mut client = create_noise_client(&key).unwrap();
        let frame = noise_handshake(&mut client);
        assert_eq!(frame[0], NOISE_PREAMBLE);
        // Length field is 2 bytes
        assert_eq!(
            u16::try_from(frame.len()).unwrap(),
            u16::from_be_bytes([frame[1], frame[2]]) + 3
        );
    }

    #[test]
    fn test_parse_noise_response_valid() {
        // Prepare a valid handshake state and message
        let key = create_key(3u8);
        let mut client = create_noise_client(&key).unwrap();
        let mut server = create_noise_server(&key).unwrap();

        // Simulate handshake
        let mut payload = vec![0u8; 65535];
        let payload_size = client.write_message(&[], &mut payload).unwrap();
        payload.truncate(payload_size);
        let mut read_data = vec![0u8; 65535];
        server.read_message(&payload, &mut read_data).unwrap();

        // Generate simulated response
        let mut write_data = vec![0u8; 65535];
        let size = server.write_message(&[], &mut write_data).unwrap();
        write_data.truncate(size);
        write_data.insert(0, ZERO_BYTE);
        parse_noise_response(write_data, &mut client).expect("Should parse valid response");
    }

    #[test]
    fn test_parse_noise_response_invalid_preamble() {
        let key = create_key(4u8);
        let mut client = create_noise_client(&key).unwrap();
        let data = vec![0xFF, 0x01, 0x02];
        let result = parse_noise_response(data, &mut client);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Connection error: Noise handshake failed: Incorrect preamble:"));
    }

    #[test]
    fn test_parse_noise_response_invalid_handshake_message() {
        let key = create_key(5u8);
        let mut client = create_noise_client(&key).unwrap();
        // Valid preamble, but invalid handshake message (random bytes)
        let data = vec![ZERO_BYTE, 0xAA, 0xBB, 0xCC];
        let result = parse_noise_response(data, &mut client);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.to_string(), "Connection error: Noise handshake failed: Noise transport error: state error: NotTurnToRead");
    }
}
