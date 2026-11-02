#![allow(
    dead_code,
    reason = "Shared fixture methods are used by different integration binaries"
)]
use base64::{Engine as _, engine::general_purpose::STANDARD};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};

pub const KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE=";

pub struct NoisePeer {
    pub socket: TcpStream,
    cipher: snow::TransportState,
}
impl NoisePeer {
    pub async fn accept(socket: TcpStream) -> Self {
        Self::accept_with_delay(socket, std::time::Duration::ZERO).await
    }
    pub async fn accept_with_delay(mut socket: TcpStream, delay: std::time::Duration) -> Self {
        let key: [u8; 32] = STANDARD.decode(KEY).unwrap().try_into().unwrap();
        let mut handshake =
            snow::Builder::new("Noise_NNpsk0_25519_ChaChaPoly_SHA256".parse().unwrap())
                .prologue(b"NoiseAPIInit\0\0")
                .unwrap()
                .psk(0, &key)
                .unwrap()
                .build_responder()
                .unwrap();
        assert_eq!(read_frame(&mut socket).await, Vec::<u8>::new());
        tokio::time::sleep(delay).await;
        write_frame(&mut socket, b"\x01test-peer\0").await;
        let request = read_frame(&mut socket).await;
        assert_eq!(request[0], 0);
        handshake.read_message(&request[1..], &mut []).unwrap();
        let mut response = [0; 256];
        let size = handshake.write_message(&[], &mut response[1..]).unwrap();
        write_frame(&mut socket, &response[..size + 1]).await;
        Self {
            socket,
            cipher: handshake.into_transport_mode().unwrap(),
        }
    }
    pub async fn read(&mut self) -> Vec<u8> {
        let frame = read_frame(&mut self.socket).await;
        let mut plain = vec![0; 65535];
        let size = self.cipher.read_message(&frame, &mut plain).unwrap();
        plain.truncate(size);
        plain
    }
    pub async fn write(&mut self, message: &[u8]) {
        self.try_write(message).await.unwrap();
    }
    pub async fn try_write(&mut self, message: &[u8]) -> std::io::Result<()> {
        let mut encrypted = vec![0; 65535];
        let size = self
            .cipher
            .write_message(message, &mut encrypted)
            .map_err(std::io::Error::other)?;
        self.socket.write_all(&[1]).await?;
        self.socket
            .write_all(&u16::try_from(size).unwrap().to_be_bytes())
            .await?;
        self.socket.write_all(&encrypted[..size]).await
    }
}
async fn read_frame(socket: &mut TcpStream) -> Vec<u8> {
    let mut header = [0; 3];
    socket.read_exact(&mut header).await.unwrap();
    assert_eq!(header[0], 1);
    let mut frame = vec![0; usize::from(u16::from_be_bytes([header[1], header[2]]))];
    socket.read_exact(&mut frame).await.unwrap();
    frame
}
async fn write_frame(socket: &mut TcpStream, data: &[u8]) {
    socket.write_all(&[1]).await.unwrap();
    socket
        .write_all(&u16::try_from(data.len()).unwrap().to_be_bytes())
        .await
        .unwrap();
    socket.write_all(data).await.unwrap();
}
