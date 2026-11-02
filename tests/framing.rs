use esphome_client::{EspHomeClient, types::EspHomeMessage};
use tokio::{
    io::AsyncWriteExt,
    net::TcpListener,
    time::{Duration, timeout},
};

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
