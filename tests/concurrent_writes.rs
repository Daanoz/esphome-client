mod support;
use esphome_client::{EspHomeClient, types::HelloRequest};
use tokio::{
    net::TcpListener,
    time::{Duration, timeout},
};

#[tokio::test]
async fn cloned_noise_writers_preserve_cipher_order() {
    timeout(Duration::from_secs(5), async {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.unwrap();
            let mut peer = support::NoisePeer::accept(socket).await;
            tokio::time::sleep(Duration::from_millis(30)).await;
            for _ in 0..64 {
                let frame = peer.read().await;
                assert_eq!(&frame[..2], &[0, 1]);
                assert_eq!(
                    usize::from(u16::from_be_bytes([frame[2], frame[3]])),
                    frame.len() - 4
                );
                assert!(frame[4..].contains(&b'a') || frame[4..].contains(&b'b'));
            }
            // Ack only after every message has been decrypted with sequential nonces.
            peer.write(&[0, 8, 0, 0]).await;
        });
        let mut client = EspHomeClient::builder()
            .address(&address.to_string())
            .key(support::KEY)
            .without_connection_setup()
            .connect()
            .await
            .unwrap();
        let first = client.write_stream();
        let second = first.clone();
        let send = |writer: esphome_client::EspHomeClientWriteStream, value: char| async move {
            for _ in 0..32 {
                writer
                    .try_write(HelloRequest {
                        client_info: value.to_string().repeat(60_000),
                        api_version_major: 1,
                        api_version_minor: 15,
                    })
                    .await
                    .unwrap();
            }
        };
        tokio::join!(send(first, 'a'), send(second, 'b'));
        assert!(matches!(
            client.try_read().await.unwrap(),
            esphome_client::types::EspHomeMessage::PingResponse(_)
        ));
        server.await.unwrap();
    })
    .await
    .unwrap();
}
