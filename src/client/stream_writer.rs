use super::session::Session;
use crate::error::{ClientError, StreamError};
use std::{fmt::Debug, sync::Arc};
use tokio::{
    io::{AsyncWrite, AsyncWriteExt as _},
    net::tcp::OwnedWriteHalf,
    sync::Mutex,
};

#[derive(Debug)]
struct NoopEncoder;
impl StreamEncoder for NoopEncoder {}

pub(crate) trait StreamEncoder: Send + Sync + Debug {
    fn validate(&self, _payload: &[u8]) -> Result<(), ClientError> {
        Ok(())
    }
    fn encode(&self, payload: Vec<u8>) -> Result<Vec<u8>, ClientError> {
        Ok(payload)
    }
}

#[derive(Debug)]
struct WriteState {
    encoder: Box<dyn StreamEncoder>,
    stream: OwnedWriteHalf,
}

#[derive(Debug, Clone)]
pub(crate) struct StreamWriter {
    state: Arc<Mutex<WriteState>>,
    session: Arc<Session>,
}

// A cancelled future may already have advanced the Noise nonce or written a
// prefix. Its socket cannot safely carry another frame.
struct WriteGuard<'a> {
    session: &'a Session,
    complete: bool,
}
impl Drop for WriteGuard<'_> {
    fn drop(&mut self) {
        if !self.complete {
            self.session.close();
        }
    }
}

async fn write_encoded<W: AsyncWrite + Unpin>(
    writer: &mut W,
    encoder: &dyn StreamEncoder,
    payload: Vec<u8>,
    session: &Session,
) -> Result<(), ClientError> {
    session.ensure_open()?;
    encoder.validate(&payload)?;
    let mut guard = WriteGuard {
        session,
        complete: false,
    };
    let frame = encoder.encode(payload)?;
    writer
        .write_all(&frame)
        .await
        .map_err(|source| StreamError::Write { source })?;
    guard.complete = true;
    Ok(())
}

impl StreamWriter {
    pub(crate) fn new(stream: OwnedWriteHalf, session: Arc<Session>) -> Self {
        Self {
            state: Arc::new(Mutex::new(WriteState {
                encoder: Box::new(NoopEncoder),
                stream,
            })),
            session,
        }
    }
    pub(crate) fn with_encoder(mut self, encoder: Box<dyn StreamEncoder>) -> Self {
        Arc::get_mut(&mut self.state)
            .expect("encoder set before writer is shared")
            .get_mut()
            .encoder = encoder;
        self
    }
    pub(crate) fn close(&self) {
        self.session.close();
    }
    pub(crate) async fn write_message(&self, payload: Vec<u8>) -> Result<(), ClientError> {
        self.session.ensure_open()?;
        let mut state = self.state.lock().await;
        let WriteState { stream, encoder } = &mut *state;
        let result = write_encoded(stream, encoder.as_ref(), payload, &self.session).await;
        drop(state);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::{
        pin::Pin,
        task::{Context, Poll},
    };
    use tokio::{
        io::AsyncReadExt as _,
        io::AsyncWrite,
        net::{TcpListener, TcpStream},
        time::{Duration, timeout},
    };

    #[derive(Default)]
    struct ShortSink {
        bytes: Vec<u8>,
        limit: usize,
        block_after: Option<usize>,
        fail: bool,
    }
    impl AsyncWrite for ShortSink {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            if self.fail {
                return Poll::Ready(Err(io::Error::other("failed")));
            }
            if self
                .block_after
                .is_some_and(|limit| self.bytes.len() >= limit)
            {
                return Poll::Pending;
            }
            let count = buf.len().min(self.limit);
            self.bytes.extend_from_slice(&buf[..count]);
            Poll::Ready(Ok(count))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    async fn session() -> (StreamReaderForTest, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let stream = TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let (peer, _) = listener.accept().await.unwrap();
        let pair = super::super::session::split(stream).unwrap();
        (pair.0, peer)
    }
    type StreamReaderForTest = super::super::stream_reader::StreamReader;

    #[tokio::test]
    async fn partial_write_sends_complete_frame() {
        let (reader, _peer) = session().await;
        let mut sink = ShortSink {
            limit: 2,
            ..ShortSink::default()
        };
        write_encoded(
            &mut sink,
            &NoopEncoder,
            vec![1, 2, 3, 4, 5],
            reader.session(),
        )
        .await
        .unwrap();
        assert_eq!(sink.bytes, [1, 2, 3, 4, 5]);
    }

    #[tokio::test]
    async fn write_zero_returns_error_and_closes_session() {
        let (reader, _peer) = session().await;
        let mut sink = ShortSink::default();
        assert!(
            write_encoded(&mut sink, &NoopEncoder, vec![1], reader.session())
                .await
                .is_err()
        );
        assert!(reader.session().is_closed());
    }

    #[tokio::test]
    async fn cancelled_write_terminates_session() {
        for first_bytes in [0, 2] {
            let (mut reader, mut peer) = session().await;
            let state = Arc::clone(reader.session());
            let mut sink = ShortSink {
                limit: 2,
                block_after: Some(first_bytes),
                ..ShortSink::default()
            };
            let (write_result, read_result) = tokio::join!(
                timeout(
                    Duration::from_millis(20),
                    write_encoded(&mut sink, &NoopEncoder, vec![1, 2, 3], &state)
                ),
                timeout(Duration::from_secs(1), reader.read_next_message())
            );
            write_result.unwrap_err();
            assert!(state.is_closed());
            read_result.unwrap().unwrap_err();
            assert_eq!(
                timeout(Duration::from_secs(1), peer.read_u8())
                    .await
                    .unwrap()
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::UnexpectedEof
            );
        }
    }

    #[tokio::test]
    async fn failed_write_terminates_session() {
        let (reader, _peer) = session().await;
        let mut sink = ShortSink {
            fail: true,
            ..ShortSink::default()
        };
        assert!(
            write_encoded(&mut sink, &NoopEncoder, vec![1], reader.session())
                .await
                .is_err()
        );
        assert!(reader.session().is_closed());
    }
}
