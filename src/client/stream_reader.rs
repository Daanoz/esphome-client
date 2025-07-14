use crate::error::{ClientError, StreamError};
use std::{fmt::Debug, io, mem};
use tokio::{io::Interest, net::tcp::OwnedReadHalf};

#[derive(Debug)]
struct NoopDecoder;
impl StreamDecoder for NoopDecoder {}

pub(crate) trait StreamDecoder: Send + Sync + Debug {
    fn decode(&self, buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
        if buffer.is_empty() {
            return Ok(None);
        }
        Ok(Some(mem::take(buffer)))
    }
}

#[derive(Debug)]
pub(crate) struct StreamReader {
    decoder: Box<dyn StreamDecoder>,
    read_stream: OwnedReadHalf,
    buffer: Vec<u8>,
}

impl StreamReader {
    pub(crate) fn new(read_stream: OwnedReadHalf) -> Self {
        Self {
            read_stream,
            decoder: Box::new(NoopDecoder),
            buffer: Vec::with_capacity(65535),
        }
    }

    pub(crate) fn with_decoder(self, decoder: Box<dyn StreamDecoder>) -> Self {
        Self {
            decoder,
            read_stream: self.read_stream,
            buffer: self.buffer,
        }
    }

    pub(crate) async fn read_next_message(&mut self) -> Result<Vec<u8>, ClientError> {
        if let Ok(Some(decoded)) = self.decoder.decode(&mut self.buffer) {
            tracing::trace!("Read {} bytes: {decoded:?}", decoded.len());
            return Ok(decoded);
        }
        loop {
            let ready = self
                .read_stream
                .ready(Interest::READABLE)
                .await
                .map_err(|e| StreamError::Read { source: e })?;
            if ready.is_readable() {
                match self.read_stream.try_read_buf(&mut self.buffer) {
                    Ok(n) if n < 1 => {}
                    Ok(_) => {
                        if let Ok(Some(decoded)) = self.decoder.decode(&mut self.buffer) {
                            tracing::trace!("Read {} bytes: {:?}", decoded.len(), decoded);
                            return Ok(decoded);
                        }
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => return Err(StreamError::Read { source: e }.into()),
                }
            }
        }
    }
}
