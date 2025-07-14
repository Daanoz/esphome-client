use esphome_client::{
    types::{EspHomeMessage, HelloRequest, HelloResponse},
    EspHomeClient,
};
use prost::Message;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{timeout, Duration},
};

const KEY: &str = "AAECAwQFBgcICRAREhMUFRYXGBkgISIjJCUmJygpMDE="; // Dummy key for testing

#[tokio::test]
async fn test_noise_connection_hello() {
    // Start mock server
    let addr = "127.0.0.1:16054";
    let mock_server = MockServer::start(addr.into());

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Build a noise connection
    let mut stream = EspHomeClient::builder()
        .address(addr)
        .timeout(Duration::from_secs(2))
        .key(KEY)
        .without_connection_setup()
        .connect()
        .await
        .expect("Failed to connect in noise mode");

    // Send a HelloRequest
    let hello = HelloRequest {
        client_info: "integration-test".to_string(),
        api_version_major: 1,
        api_version_minor: 10,
    };
    timeout(Duration::from_secs(2), stream.try_write(hello))
        .await
        .expect("Timeout writing for HelloRequest")
        .expect("Failed to send HelloRequest");

    // Read the HelloResponse
    let response = timeout(Duration::from_secs(2), stream.try_read())
        .await
        .expect("Timeout waiting for HelloResponse")
        .expect("Failed to read HelloResponse");

    match response {
        EspHomeMessage::HelloResponse(_) => {
            // Success
        }
        other => panic!("Expected HelloResponse, got {:?}", other),
    }

    mock_server.close();
}

struct MockServer {
    handle: tokio::task::JoinHandle<()>,
}
impl MockServer {
    fn start(addr: String) -> Self {
        MockServer {
            handle: tokio::spawn(start_mock_server(addr)),
        }
    }
    fn close(self) {
        self.handle.abort();
    }
}
impl Drop for MockServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

async fn start_mock_server(addr: String) {
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind mock server");
    loop {
        let (mut socket, _) = listener
            .accept()
            .await
            .expect("Failed to accept connection");

        let mut noise_responder = noise_responder();

        // Handle the Noise handshake
        read_noise_hello(&mut socket).await;
        read_noise_handshake(&mut socket, &mut noise_responder).await;
        write_server_and_mac(&mut socket).await;
        write_noise_response(&mut socket, &mut noise_responder).await;
        assert!(noise_responder.is_handshake_finished());
        let mut noise_responder = noise_responder
            .into_transport_mode()
            .expect("Transport mode");

        // Read HelloRequest
        let mut len_buf = [0u8; 3];
        socket
            .read_exact(&mut len_buf)
            .await
            .expect("Failed to read Header");
        assert_eq!(len_buf[0], 0x01); // Ensure preamble is 1 (Noise mode)
        let len = u16::from_be_bytes([len_buf[1], len_buf[2]]) as usize;

        let mut buf = vec![0u8; len];
        socket
            .read_exact(&mut buf)
            .await
            .expect("Failed to read HelloRequest");

        let mut payload = vec![0u8; 65535];
        let size = noise_responder
            .read_message(&buf, &mut payload)
            .expect("Failed to decode payload");
        payload.truncate(size);

        assert_eq!(u16::from_be_bytes([payload[0], payload[1]]), 1); // HelloRequest type ID
        let len = u16::from_be_bytes([payload[2], payload[3]]) as usize;
        assert!(
            HelloRequest::decode(&payload[4..len]).is_ok(),
            "Failed to decode HelloRequest"
        );

        // Respond with HelloResponse
        let response = HelloResponse {
            name: "mock-server".to_string(),
            server_info: "mock-server".to_string(),
            api_version_major: 1,
            api_version_minor: 10,
        };
        let mut out_buf: Vec<u8> = vec![];
        response
            .encode(&mut out_buf)
            .expect("Encoding HelloResponse failed");

        let mut payload = vec![0u8; 65535];
        let size = noise_responder
            .write_message(
                &[
                    (2_u16).to_be_bytes().to_vec(), // Message type ID for HelloResponse
                    (out_buf.len() as u16).to_be_bytes().to_vec(), // Length of the message
                    out_buf,
                ]
                .concat(),
                &mut payload,
            )
            .expect("Encoding handshake");
        payload.truncate(size);

        send_noise_frame(&mut socket, &payload).await;

        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
}

async fn read_noise_hello(stream: &mut TcpStream) {
    let mut buf = [0u8; 3];
    stream.read_exact(&mut buf).await.expect("Bytes");
    assert_eq!(&buf, b"\x01\x00\x00");
}

async fn read_noise_handshake(stream: &mut TcpStream, noise_responder: &mut snow::HandshakeState) {
    let frame = read_next_frame(stream).await;

    assert_eq!(frame[0], 0x00); // ZERO byte separator

    let mut payload = vec![];
    noise_responder
        .read_message(&frame[1..], &mut payload)
        .expect("Handshake read");
}

async fn write_server_and_mac(stream: &mut TcpStream) {
    let payload = b"\x01ServerName\x00abcdef012345\x00";

    send_noise_frame(stream, payload).await;
}

async fn write_noise_response(stream: &mut TcpStream, noise_responder: &mut snow::HandshakeState) {
    let mut payload = vec![0u8; 65535];
    let size = noise_responder
        .write_message(&[], &mut payload)
        .expect("Encoding handshake");
    payload.truncate(size);
    payload.insert(0, 0x00);

    send_noise_frame(stream, &payload).await;
}

async fn read_next_frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut len_buf = [0u8; 3];
    stream.read_exact(&mut len_buf).await.expect("Header bytes");

    assert_eq!(len_buf[0], 0x01); // Preamble for Noise
    let len = u16::from_be_bytes([len_buf[1], len_buf[2]]) as usize;

    let mut frame = vec![0u8; len];
    stream.read_exact(&mut frame).await.expect("Payload bytes");
    frame
}

async fn send_noise_frame(stream: &mut TcpStream, payload: &[u8]) {
    let len = payload.len() as u16;
    stream
        .write_all(&[0x01])
        .await
        .expect("Write Noise preamble");
    stream
        .write_all(&len.to_be_bytes())
        .await
        .expect("Write length");
    stream.write_all(payload).await.expect("Write payload");
}

fn noise_responder() -> snow::HandshakeState {
    use base64::{engine::general_purpose, Engine as _};
    let key_bytes = general_purpose::STANDARD
        .decode(KEY)
        .expect("Valid base64 key");
    if key_bytes.len() != 32 {
        panic!("Invalid PSK length");
    }
    snow::Builder::new(
        "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
            .parse()
            .expect("Valid encryption protocol"),
    )
    .prologue(b"NoiseAPIInit\x00\x00")
    .psk(0, &key_bytes)
    .build_responder()
    .expect("Failed to setup snow initiator")
}
