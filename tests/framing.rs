use esphome_client::{EspHomeClient, types::EspHomeMessage};
use tokio::{
    io::AsyncWriteExt,
    net::TcpListener,
    time::{Duration, timeout},
};

#[tokio::test]
async fn close_invalidates_existing_write_clones() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let mut client = EspHomeClient::builder()
            .address(&address.to_string())
            .without_connection_setup()
            .connect()
            .await
            .unwrap();
        let (_peer, _) = listener.accept().await.unwrap();
        let writer = client.write_stream();
        // Confirm this is a usable session before close.
        client
            .try_write(esphome_client::types::PingResponse {})
            .await
            .unwrap();
        client.close().await.unwrap();
        assert!(
            writer
                .try_write(esphome_client::types::PingResponse {})
                .await
                .is_err()
        );
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn coalesced_frames_are_preserved() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let peer = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            // Two independently encoded empty PingResponse messages.
            socket.write_all(&[0, 0, 8, 0, 0, 8]).await.unwrap();
        });
        let mut client = EspHomeClient::builder()
            .address(&address.to_string())
            .without_connection_setup()
            .connect()
            .await
            .unwrap();
        assert!(matches!(
            client.try_read().await.unwrap(),
            EspHomeMessage::PingResponse(_)
        ));
        assert!(matches!(
            client.try_read().await.unwrap(),
            EspHomeMessage::PingResponse(_)
        ));
        assert!(client.try_read().await.is_err());
        peer.await.unwrap();
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelled_read_preserves_partial_frame() {
    timeout(Duration::from_secs(2), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut client = EspHomeClient::builder()
            .address(&listener.local_addr().unwrap().to_string())
            .without_connection_setup()
            .connect()
            .await
            .unwrap();
        let (mut peer, _) = listener.accept().await.unwrap();
        // GetTimeResponse.epoch_seconds is fixed32, tag 0x0d.
        peer.write_all(&[0, 5, 37, 13, 42, 0]).await.unwrap();
        assert!(
            timeout(Duration::from_millis(20), client.try_read())
                .await
                .is_err()
        );
        peer.write_all(&[0, 0]).await.unwrap();
        match client.try_read().await.unwrap() {
            EspHomeMessage::GetTimeResponse(value) => assert_eq!(value.epoch_seconds, 42),
            other => panic!("unexpected {other:?}"),
        }
    })
    .await
    .unwrap();
}
