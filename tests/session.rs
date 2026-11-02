use esphome_client::{EspHomeClient, error::ClientError};
mod support;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::{Duration, sleep, timeout},
};

async fn read_message(socket: &mut TcpStream) -> Vec<u8> {
    assert_eq!(socket.read_u8().await.unwrap(), 0);
    let mut size = 0usize;
    for shift in [0, 7, 14] {
        let value = socket.read_u8().await.unwrap();
        size |= usize::from(value & 127) << shift;
        if value & 128 == 0 {
            break;
        }
    }
    let id = socket.read_u8().await.unwrap();
    let mut data = vec![0; size];
    socket.read_exact(&mut data).await.unwrap();
    let mut result = vec![id];
    result.extend(data);
    result
}

// Independently encoded HelloResponse API1.13, server_info=test, name=node.
const HELLO: &[u8] = &[
    0, 16, 2, 8, 1, 16, 13, 26, 4, b't', b'e', b's', b't', 34, 4, b'n', b'o', b'd', b'e',
];

#[tokio::test]
async fn hello_metadata_survives_connect() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_message(&mut socket).await;
            let mut hello = HELLO.to_vec();
            hello[6] = 15;
            socket.write_all(&hello).await.unwrap();
            socket.write_all(&[0, 0, 8]).await.unwrap();
        });
        let mut client = EspHomeClient::builder()
            .address(&address.to_string())
            .connect()
            .await
            .unwrap();
        let info = client.connection_info().unwrap();
        assert_eq!((info.api_version_major, info.api_version_minor), (1, 15));
        assert_eq!(info.server_info, "test");
        assert_eq!(info.name, "node");
        assert!(matches!(
            client.try_read().await.unwrap(),
            esphome_client::types::EspHomeMessage::PingResponse(_)
        ));
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn manual_setup_does_not_consume_hello_or_fabricate_metadata() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = EspHomeClient::builder()
            .address(&listener.local_addr().unwrap().to_string())
            .without_connection_setup()
            .connect()
            .await
            .unwrap();
        let (mut peer, _) = listener.accept().await.unwrap();
        assert!(client.connection_info().is_none());
        peer.write_all(HELLO).await.unwrap();
        assert!(matches!(
            client.try_read().await.unwrap(),
            esphome_client::types::EspHomeMessage::HelloResponse(_)
        ));
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn different_api_major_is_rejected() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_message(&mut socket).await;
            let mut hello = HELLO.to_vec();
            hello[4] = 2;
            socket.write_all(&hello).await.unwrap();
            assert!(socket.read_u8().await.is_err());
        });
        assert!(matches!(
            EspHomeClient::builder()
                .address(&address.to_string())
                .connect()
                .await,
            Err(ClientError::ProtocolMismatch { .. })
        ));
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn setup_timeout_includes_hello() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            assert_eq!(read_message(&mut socket).await[0], 1);
            assert!(socket.read_u8().await.is_err());
        });
        let result = EspHomeClient::builder()
            .address(&addr.to_string())
            .timeout(Duration::from_millis(100))
            .connect()
            .await;
        assert!(matches!(result, Err(ClientError::Timeout { .. })));
        server.await.unwrap();
    })
    .await
    .expect("setup was not bounded");
}

#[tokio::test]
async fn authentication_uses_same_deadline() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_message(&mut socket).await;
            socket.write_all(HELLO).await.unwrap();
            assert_eq!(read_message(&mut socket).await[0], 3);
            assert!(socket.read_u8().await.is_err());
        });
        let result = EspHomeClient::builder()
            .address(&addr.to_string())
            .password("test")
            .timeout(Duration::from_millis(100))
            .connect()
            .await;
        assert!(matches!(result, Err(ClientError::Timeout { .. })));
        server.await.unwrap();
    })
    .await
    .expect("authentication was not bounded");
}

#[tokio::test]
async fn setup_deadline_is_shared_across_phases() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            read_message(&mut socket).await;
            sleep(Duration::from_millis(60)).await;
            socket.write_all(HELLO).await.unwrap();
            read_message(&mut socket).await;
            sleep(Duration::from_millis(60)).await;
            // A per-stage100ms timeout would accept this response; one shared deadline must not.
            let _response = socket.write_all(&[0, 0, 4]).await;
        });
        let result = EspHomeClient::builder()
            .address(&addr.to_string())
            .password("test")
            .timeout(Duration::from_millis(100))
            .connect()
            .await;
        assert!(matches!(result, Err(ClientError::Timeout { .. })));
        server.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn noise_setup_timeout_includes_hello() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut peer = support::NoisePeer::accept(socket).await;
            assert_eq!(&peer.read().await[..2], &[0, 1]);
            assert!(peer.socket.read_u8().await.is_err());
        });
        let result = EspHomeClient::builder()
            .address(&address.to_string())
            .key(support::KEY)
            .timeout(Duration::from_millis(100))
            .connect()
            .await;
        assert!(matches!(result, Err(ClientError::Timeout { .. })));
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn noise_and_hello_share_one_deadline() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut peer =
                support::NoisePeer::accept_with_delay(socket, Duration::from_millis(60)).await;
            assert_eq!(&peer.read().await[..2], &[0, 1]);
            sleep(Duration::from_millis(60)).await;
            let _response = peer.try_write(&[0, 2, 0, 4, 8, 1, 16, 15]).await;
        });
        let result = EspHomeClient::builder()
            .address(&address.to_string())
            .key(support::KEY)
            .timeout(Duration::from_millis(100))
            .connect()
            .await;
        assert!(matches!(result, Err(ClientError::Timeout { .. })));
        peer.await.unwrap();
    })
    .await
    .unwrap();
}
