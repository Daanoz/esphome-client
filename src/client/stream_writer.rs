use std::{fmt::Debug, io, sync::Arc};
use tokio::{io::Interest, net::tcp::OwnedWriteHalf};

use crate::error::{ClientError, StreamError};

#[derive(Debug)]
struct NoopEncoder;
impl StreamEncoder for NoopEncoder {}

pub(crate) trait StreamEncoder: Send + Sync + Debug {
    fn encode(&self, payload: Vec<u8>) -> Result<Vec<u8>, ClientError> {
        Ok(payload)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct StreamWriter {
    encoder: Arc<Box<dyn StreamEncoder>>,
    write_stream: Arc<OwnedWriteHalf>,
}

impl StreamWriter {
    pub(crate) fn new(write_stream: OwnedWriteHalf) -> Self {
        let encoder: Box<dyn StreamEncoder> = Box::new(NoopEncoder);
        Self {
            write_stream: write_stream.into(),
            encoder: encoder.into(),
        }
    }

    pub(crate) fn with_encoder(self, encoder: Box<dyn StreamEncoder>) -> Self {
        Self {
            encoder: encoder.into(),
            write_stream: self.write_stream,
        }
    }

    pub(crate) async fn write_message(&self, payload: Vec<u8>) -> Result<(), ClientError> {
        let payload = self.encoder.encode(payload)?;
        loop {
            let ready = self
                .write_stream
                .ready(Interest::WRITABLE)
                .await
                .map_err(|e| StreamError::Write { source: e })?;
            if ready.is_writable() {
                match self.write_stream.try_write(&payload) {
                    Ok(n) => {
                        tracing::trace!("Wrote {n} bytes: {payload:?}");
                        return Ok(());
                    }
                    Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                    Err(e) => {
                        return Err(StreamError::Write { source: e }.into());
                    }
                }
            }
        }
    }
}
