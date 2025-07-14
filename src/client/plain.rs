use tokio::net::TcpStream;

use super::{
    noise::NOISE_PREAMBLE,
    stream_reader::{StreamDecoder, StreamReader},
    stream_writer::{StreamEncoder, StreamWriter},
    StreamPair,
};
use crate::error::{ClientError, ConnectionError, ProtocolError, StreamError};

pub(super) const PLAIN_PREAMBLE: u8 = 0x00;

pub(crate) async fn connect(addr: &str) -> Result<StreamPair, ClientError> {
    let (read_stream, write_stream) = TcpStream::connect(addr)
        .await
        .map_err(|e| ConnectionError::TcpConnect {
            address: addr.to_owned(),
            source: e,
        })?
        .into_split();
    tracing::debug!("Tcp connection established to {addr}");
    Ok((
        StreamReader::new(read_stream).with_decoder(Box::new(PlainDecoder)),
        StreamWriter::new(write_stream).with_encoder(Box::new(PlainEncoder)),
    ))
}

#[derive(Debug)]
struct PlainDecoder;
impl StreamDecoder for PlainDecoder {
    fn decode(&self, buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
        read_frame_from_buffer(buffer)
    }
}

#[derive(Debug)]
struct PlainEncoder;
impl StreamEncoder for PlainEncoder {
    fn encode(&self, payload: Vec<u8>) -> Result<Vec<u8>, ClientError> {
        create_frame(&payload)
    }
}

/// Create a frame with the given payload, including the preamble and length.
fn create_frame(payload: &[u8]) -> Result<Vec<u8>, ClientError> {
    // Plain payload are structured differently than Noise payloads.
    // Noise payloads have 2 bytes for the type and then 2 bytes for the length
    // Plain payloads use leb128 compression for first the length, then the type
    if payload.len() < 4 {
        return Err(StreamError::InvalidFrame {
            reason: "Payload must be at least 4 bytes long".to_owned(),
        }
        .into());
    }
    let type_id = u16::from_be_bytes([payload[0], payload[1]]);
    let frame_len = u16::from_be_bytes([payload[2], payload[3]]);
    Ok([
        vec![PLAIN_PREAMBLE],
        convert_to_leb128(frame_len),
        convert_to_leb128(type_id),
        payload[4..].to_vec(),
    ]
    .concat())
}

/// Attempts to read a frame from the buffer.
fn read_frame_from_buffer(buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
    if buffer.len() < 3 {
        return Ok(None);
    }
    let preamble = buffer[0];
    match preamble {
        PLAIN_PREAMBLE => {}
        NOISE_PREAMBLE => {
            return Err(ProtocolError::UnexpectedEncryption.into());
        }
        _ => {
            return Err(StreamError::InvalidFrame {
                reason: format!("Invalid preamble: {preamble}"),
            }
            .into());
        }
    }
    let (frame_len, next_index) = match convert_from_leb128(buffer, 1) {
        Some((len, index)) => (usize::from(len), index),
        None => return Ok(None),
    };
    let Some((type_id, next_index)) = convert_from_leb128(buffer, next_index) else {
        return Ok(None);
    };
    if buffer.len() < next_index + frame_len {
        tracing::debug!(
            "Waiting for more data, expected {} bytes, got {}",
            frame_len,
            buffer.len()
        );
        return Ok(None);
    }
    let frame = buffer
        .drain(..frame_len + next_index)
        .skip(next_index)
        .collect();
    let frame_len = u16::try_from(frame_len).map_err(|_e| StreamError::FrameTooLarge {
        size: frame_len,
        #[allow(clippy::as_conversions, reason = "u16:MAX should always fit in usize")]
        max_size: u16::MAX as usize,
    })?;
    // Reconstruct frame as it came from noise encrypted stream, 2 bytes for type and 2 bytes for length
    Ok(Some(
        [
            type_id.to_be_bytes().to_vec(),
            frame_len.to_be_bytes().to_vec(),
            frame,
        ]
        .concat(),
    ))
}

fn convert_to_leb128(mut value: u16) -> Vec<u8> {
    if value <= 0x7F {
        return vec![u8::try_from(value).expect("u8")];
    }

    let mut result = Vec::new();

    while value != 0 {
        let mut temp = u8::try_from(value & 0x7F).expect("u8");
        value >>= 7;
        if value != 0 {
            temp |= 0x80;
        }
        result.push(temp);
    }

    result
}

fn convert_from_leb128(payload: &[u8], start_pos: usize) -> Option<(u16, usize)> {
    let mut result: u16 = 0;
    let mut shift = 0;

    for (index, byte) in payload.iter().enumerate().skip(start_pos) {
        let value = u16::from(byte & 0x7F);
        result |= value << shift;

        if byte & 0x80 == 0 {
            return Some((result, index + 1));
        }

        shift += 7;

        if shift >= 16 {
            // Prevent overflow for u16
            return None;
        }
    }

    None // Incomplete encoding
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_convert_to_leb128_and_from_leb128() {
        let values = [0u16, 1, 127, 128, 255, 300, 16383, 16384, u16::MAX];
        for &val in &values {
            let leb = convert_to_leb128(val);
            let (decoded, next_index) = convert_from_leb128(&leb, 0).expect("Should decode");
            assert_eq!(decoded, val);
            assert_eq!(next_index, leb.len());
        }
    }

    #[test]
    fn test_create_frame_and_read_frame_from_buffer() {
        let type_id: u16 = 0x1234;
        let payload_data = vec![1, 2, 3, 4, 5, 6];
        let frame_len = u16::try_from(payload_data.len()).expect("payload too large");
        let mut payload = Vec::new();
        payload.extend_from_slice(&type_id.to_be_bytes());
        payload.extend_from_slice(&frame_len.to_be_bytes());
        payload.extend_from_slice(&payload_data);

        let mut buffer = create_frame(&payload).expect("Frame should be created");

        let decoded = read_frame_from_buffer(&mut buffer)
            .expect("Should decode")
            .expect("Should have frame");
        // The decoded frame should reconstruct the original type_id, frame_len, and payload_data
        assert_eq!(&decoded[0..2], &type_id.to_be_bytes());
        assert_eq!(&decoded[2..4], &frame_len.to_be_bytes());
        assert_eq!(&decoded[4..], &payload_data);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_create_frame_with_short_payload() {
        let payload = vec![1, 2, 3]; // less than 4 bytes
        let result = create_frame(&payload);
        result.unwrap_err();
    }

    #[test]
    fn test_read_frame_from_buffer_with_noise_preamble() {
        let mut buffer = vec![NOISE_PREAMBLE, 0x01, 0x02, 0x03];
        let result = read_frame_from_buffer(&mut buffer);
        result.unwrap_err();
    }

    #[test]
    fn test_read_frame_from_buffer_with_invalid_preamble() {
        let mut buffer = vec![0xFF, 0x01, 0x02, 0x03];
        let result = read_frame_from_buffer(&mut buffer);
        result.unwrap_err();
    }

    #[test]
    fn test_read_frame_from_buffer_incomplete_leb128() {
        // Only preamble and one byte, not enough for length/type
        let mut buffer = vec![PLAIN_PREAMBLE, 0x81];
        let result = read_frame_from_buffer(&mut buffer);
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_read_frame_from_buffer_waits_for_more_data() {
        // Frame length is 10, but only 5 bytes of payload present
        let type_id: u16 = 0x1234;
        let frame_len: u16 = 10;
        let mut frame = vec![PLAIN_PREAMBLE];
        frame.extend(convert_to_leb128(frame_len));
        frame.extend(convert_to_leb128(type_id));
        frame.extend(vec![0u8; 5]); // not enough data

        let mut buffer = frame;
        let result = read_frame_from_buffer(&mut buffer);
        assert!(result.unwrap().is_none());
    }
}
