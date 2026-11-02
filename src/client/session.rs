use super::{StreamPair, stream_reader::StreamReader, stream_writer::StreamWriter};
use crate::error::{ClientError, StreamError};
use std::{
    net::{Shutdown, TcpStream as StdTcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::net::TcpStream;

/// One control handle for both halves and every write clone. shutdown wakes
/// the OS-backed reader without requiring a runtime task in a Drop guard.
#[derive(Debug)]
pub(super) struct Session {
    socket: StdTcpStream,
    closed: AtomicBool,
}
impl Session {
    pub(super) fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }
    pub(super) fn ensure_open(&self) -> Result<(), ClientError> {
        if self.is_closed() {
            return Err(StreamError::Closed.into());
        }
        Ok(())
    }
    pub(super) fn close(&self) {
        if !self.closed.swap(true, Ordering::AcqRel) {
            drop(self.socket.shutdown(Shutdown::Both));
        }
    }
}
pub(super) fn split(stream: TcpStream) -> Result<StreamPair, ClientError> {
    let stream = stream
        .into_std()
        .map_err(|source| StreamError::Read { source })?;
    let control = stream
        .try_clone()
        .map_err(|source| StreamError::Read { source })?;
    let stream = TcpStream::from_std(stream).map_err(|source| StreamError::Read { source })?;
    let session = Arc::new(Session {
        socket: control,
        closed: AtomicBool::new(false),
    });
    let (reader, writer) = stream.into_split();
    Ok((
        StreamReader::new(reader, Arc::clone(&session)),
        StreamWriter::new(writer, session),
    ))
}
