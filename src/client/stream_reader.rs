use super::session::Session;
use crate::error::{ClientError, StreamError};
use std::{fmt::Debug, io, mem, sync::Arc};
use tokio::{io::Interest, net::tcp::OwnedReadHalf};

const MAX_BUFFERED_BYTES: usize = 65_544;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::AsyncWriteExt as _,
        net::{TcpListener, TcpStream},
        time::{Duration, timeout},
    };

    #[derive(Debug)]
    struct Reject;
    impl StreamDecoder for Reject {
        fn decode(&self, buffer: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
            if buffer.is_empty() {
                return Ok(None);
            }
            Err(StreamError::InvalidFrame {
                reason: "rejected".into(),
            }
            .into())
        }
    }
    #[derive(Debug)]
    struct Incomplete;
    impl StreamDecoder for Incomplete {
        fn decode(&self, _: &mut Vec<u8>) -> Result<Option<Vec<u8>>, ClientError> {
            Ok(None)
        }
    }

    async fn pair() -> (StreamReader, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (peer, _) = listener.accept().await.unwrap();
        (super::super::session::split(client).unwrap().0, peer)
    }

    #[tokio::test]
    async fn decoder_error_is_returned_immediately() {
        let (reader, mut peer) = pair().await;
        let mut reader = reader.with_decoder(Box::new(Reject));
        peer.write_all(&[255]).await.unwrap();
        let result = timeout(Duration::from_secs(1), reader.read_next_message())
            .await
            .expect("decoder error was swallowed");
        assert!(matches!(
            result,
            Err(ClientError::Stream(StreamError::InvalidFrame { .. }))
        ));
    }

    #[tokio::test]
    async fn buffer_limit_is_enforced() {
        let (reader, mut peer) = pair().await;
        let mut reader = reader.with_decoder(Box::new(Incomplete));
        peer.write_all(&vec![0; 70_000]).await.unwrap();
        peer.shutdown().await.unwrap();
        let result = timeout(Duration::from_secs(1), reader.read_next_message())
            .await
            .unwrap();
        assert!(matches!(
            result,
            Err(ClientError::Stream(StreamError::FrameTooLarge { .. }))
        ));
    }
}

#[derive(Debug)]
pub(crate) struct StreamReader {
    decoder: Box<dyn StreamDecoder>,
    read_stream: OwnedReadHalf,
    buffer: Vec<u8>,
    session: Arc<Session>,
}

impl StreamReader {
    pub(crate) fn new(read_stream: OwnedReadHalf, session: Arc<Session>) -> Self {
        Self {
            read_stream,
            decoder: Box::new(NoopDecoder),
            buffer: Vec::with_capacity(65535),
            session,
        }
    }

    pub(crate) fn with_decoder(mut self, decoder: Box<dyn StreamDecoder>) -> Self {
        self.decoder = decoder;
        self
    }
    #[cfg(test)]
    pub(crate) const fn session(&self) -> &Arc<Session> {
        &self.session
    }
    pub(crate) async fn read_next_message(&mut self) -> Result<Vec<u8>, ClientError> {
        self.session.ensure_open()?;
        let result = self.read_inner().await;
        if result.is_err() {
            self.session.close();
        }
        result
    }
    async fn read_inner(&mut self) -> Result<Vec<u8>, ClientError> {
        if let Some(decoded) = self.decoder.decode(&mut self.buffer)? {
            tracing::trace!("Read {} bytes: {decoded:?}", decoded.len());
            return Ok(decoded);
        }
        loop {
            if self.buffer.len() >= MAX_BUFFERED_BYTES {
                return Err(StreamError::FrameTooLarge {
                    size: self.buffer.len(),
                    max_size: MAX_BUFFERED_BYTES,
                }
                .into());
            }
            self.session.ensure_open()?;
            let ready = self
                .read_stream
                .ready(Interest::READABLE)
                .await
                .map_err(|e| StreamError::Read { source: e })?;
            if ready.is_readable() {
                let mut chunk = [0; 8192];
                let available = chunk.len().min(MAX_BUFFERED_BYTES - self.buffer.len());
                match self.read_stream.try_read(&mut chunk[..available]) {
                    Ok(0) => {
                        return Err(StreamError::Read {
                            source: io::Error::new(
                                io::ErrorKind::UnexpectedEof,
                                "connection closed by remote",
                            ),
                        }
                        .into());
                    }
                    Ok(count) => {
                        self.buffer.extend_from_slice(&chunk[..count]);
                        if let Some(decoded) = self.decoder.decode(&mut self.buffer)? {
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

impl Drop for StreamReader {
    fn drop(&mut self) {
        self.session.close();
    }
}
