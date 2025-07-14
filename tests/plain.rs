use esphome_client::{
    types::{EspHomeMessage, HelloRequest, HelloResponse},
    EspHomeClient,
};
use prost::Message;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{timeout, Duration},
};

#[tokio::test]
async fn test_plain_connection_hello() {
    // Start mock server
    let addr = "127.0.0.1:16053";
    let mock_server = MockServer::start(addr.into());

    // Give the server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Build a plain connection (no key)
    let mut stream = EspHomeClient::builder()
        .address(addr)
        .timeout(Duration::from_secs(2))
        .without_connection_setup()
        .connect()
        .await
        .expect("Failed to connect in plain mode");

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

        // Read HelloRequest
        let mut len_buf = [0u8; 3];
        if socket.read_exact(&mut len_buf).await.is_err() {
            return;
        }
        assert_eq!(len_buf[0], 0); // Ensure preamble is 0 (Plain mode)
        let len = len_buf[1] as usize;
        assert_eq!(len_buf[2], 1); // Message type ID for HelloRequest
        let mut buf = vec![0u8; len];
        if socket.read_exact(&mut buf).await.is_err() {
            return;
        }
        assert!(
            HelloRequest::decode(buf.as_slice()).is_ok(),
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
        socket
            .write_all(
                &[
                    [0].to_vec(),                            // Preamble for plain mode
                    convert_to_leb128(out_buf.len() as u16), // Length of the message
                    [2].to_vec(),                            // Message type ID for HelloResponse
                    out_buf,
                ]
                .concat(),
            )
            .await
            .expect("Send HelloResponse");
    }
}

fn convert_to_leb128(mut value: u16) -> Vec<u8> {
    if value <= 0x7F {
        return vec![value as u8];
    }

    let mut result = Vec::new();

    while value != 0 {
        let mut temp = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            temp |= 0x80;
        }
        result.push(temp);
    }

    result
}
