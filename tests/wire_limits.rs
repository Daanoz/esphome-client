mod support;
use esphome_client::{
    EspHomeClient,
    types::{EspHomeMessage, HelloRequest, PingResponse},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    time::{Duration, timeout},
};

fn hello_with_payload_size(size: usize) -> HelloRequest {
    // Field1: one-byte tag + three-byte string length + string bytes.
    HelloRequest {
        client_info: "x".repeat(size - 4),
        api_version_major: 0,
        api_version_minor: 0,
    }
}

#[test]
fn encoded_size_is_checked_without_infallible_conversion() {
    let message: EspHomeMessage = hello_with_payload_size(65536).into();
    assert!(message.encode_wire().is_err());
}

#[tokio::test]
async fn outgoing_limits_preserve_session_and_nonce() {
    timeout(Duration::from_secs(5), async {
        for encrypted in [false, true] {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address = listener.local_addr().unwrap();
            let limit = if encrypted { 65515 } else { 65535 };
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                if encrypted {
                    let mut peer = support::NoisePeer::accept(socket).await;
                    assert_eq!(peer.read().await.len(), limit + 4);
                    assert_eq!(peer.read().await, [0, 8, 0, 0]);
                    peer.write(&[0, 8, 0, 0]).await;
                } else {
                    let mut frame = vec![0; limit + 5]; // preamble+3-byte size+1-byte type
                    socket.read_exact(&mut frame).await.unwrap();
                    assert_eq!(&frame[..5], &[0, 255, 255, 3, 1]);
                    let mut ping = [0; 3];
                    socket.read_exact(&mut ping).await.unwrap();
                    assert_eq!(ping, [0, 0, 8]);
                    socket.write_all(&[0, 0, 8]).await.unwrap();
                }
            });
            let builder = EspHomeClient::builder()
                .address(&address.to_string())
                .without_connection_setup();
            let mut client = if encrypted {
                builder.key(support::KEY)
            } else {
                builder
            }
            .connect()
            .await
            .unwrap();
            client
                .try_write(hello_with_payload_size(limit))
                .await
                .unwrap();
            assert!(
                client
                    .try_write(hello_with_payload_size(limit + 1))
                    .await
                    .is_err()
            );
            let writer = client.write_stream();
            assert!(
                writer
                    .try_write(hello_with_payload_size(limit + 1))
                    .await
                    .is_err()
            );
            writer.try_write(PingResponse {}).await.unwrap();
            assert!(matches!(
                client.try_read().await.unwrap(),
                EspHomeMessage::PingResponse(_)
            ));
            server.await.unwrap();
        }
    })
    .await
    .unwrap();
}
