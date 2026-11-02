#![cfg(not(any(
    feature = "api-1-8",
    feature = "api-1-9",
    feature = "api-1-10",
    feature = "api-1-12",
    feature = "api-1-13",
    feature = "api-1-14"
)))]
mod support;
use esphome_client::{
    EspHomeClient, MessageDecodeError,
    error::{ClientError, ProtocolError},
    types::{
        BluetoothScannerSetModeRequest, DeviceCapabilitiesRequest, DeviceInfoRequest,
        EspHomeMessage, SubscribeBluetoothLeAdvertisementsRequest,
    },
};
use tokio::{
    net::TcpListener,
    time::{Duration, timeout},
};

// Literal protobuf fixture from the common API1.15/1.16 wire definition:
// batch1, MAC aa:bb:cc:dd:ee:ff, RSSI -60 (zigzag119), random address1,
// flags AD and BTHome v2 service data, temperature25C/humidity50.55%.
const AD: &[u8] = &[2, 1, 6, 10, 22, 210, 252, 64, 2, 196, 9, 3, 191, 19];
const RAW: &[u8] = &[
    0, 93, 0, 30, 10, 28, 8, 255, 221, 247, 230, 188, 215, 42, 16, 119, 24, 1, 34, 14, 2, 1, 6, 10,
    22, 210, 252, 64, 2, 196, 9, 3, 191, 19,
];

#[tokio::test]
async fn ble_raw_sessions_across_api_versions_and_unknown_messages() {
    timeout(Duration::from_secs(5), async {
        for minor in [15,16] {
            let listener=TcpListener::bind("127.0.0.1:0").await.unwrap();
            let address=listener.local_addr().unwrap();
            let peer=tokio::spawn(async move {
                let (socket,_)=listener.accept().await.unwrap();
                let mut peer=support::NoisePeer::accept(socket).await;
                assert_eq!(&peer.read().await[..2], &[0,1]);
                peer.write(&[0,2,0,4,8,1,16,minor]).await;
                assert_eq!(peer.read().await,[0,9,0,0]);
                peer.write(&[0,10,0,6,18,4,b'n',b'o',b'd',b'e']).await;
                assert_eq!(peer.read().await,[0,149,0,0]);
                // Capabilities: passive scan | raw advertisements | state and mode.
                peer.write(&[0,150,0,4,10,2,8,97]).await;
                // RAW_ADVERTISEMENTS subscription flag is1 in ESPHome2026.8.2 bluetooth_proxy.h.
                assert_eq!(peer.read().await,[0,66,0,2,8,1]);
                assert_eq!(peer.read().await,[0,127,0,2,8,1]);
                peer.write(&[0,126,0,4,8,2,16,1]).await;
                peer.write(RAW).await;
                peer.write(&[255,255,0,0]).await;
                peer.write(RAW).await;
            });
            let mut client=EspHomeClient::builder().address(&address.to_string()).key(support::KEY).connect().await.unwrap();
            assert_eq!(client.connection_info().unwrap().api_version_minor,u32::from(minor));
            client.try_write(DeviceInfoRequest {}).await.unwrap();
            assert!(matches!(client.try_read().await.unwrap(),EspHomeMessage::DeviceInfoResponse(info) if info.name=="node"));
            client.try_write(DeviceCapabilitiesRequest {}).await.unwrap();
            assert!(matches!(client.try_read().await.unwrap(),EspHomeMessage::DeviceCapabilitiesResponse(cap) if cap.bluetooth_proxy.as_ref().unwrap().feature_flags==97));
            client.try_write(SubscribeBluetoothLeAdvertisementsRequest {flags:1}).await.unwrap();
            client.try_write(BluetoothScannerSetModeRequest {mode:1}).await.unwrap();
            assert!(matches!(client.try_read().await.unwrap(),EspHomeMessage::BluetoothScannerStateResponse(state) if state.state==2 && state.mode==1));
            for batch in 0..2 {
                if batch==1 {
                    assert!(matches!(client.try_read().await,Err(ClientError::Protocol(ProtocolError::MessageDecode(MessageDecodeError::UnknownMessageType {type_id:65535})))));
                }
                match client.try_read().await.unwrap() {
                    EspHomeMessage::BluetoothLeRawAdvertisementsResponse(response) => {
                        assert_eq!(response.advertisements.len(),1);
                        let adv=&response.advertisements[0];
                        assert_eq!(adv.address,0xAABB_CCDD_EEFF);
                        assert_eq!(adv.rssi,-60); assert_eq!(adv.address_type,1); assert_eq!(adv.data,AD);
                    }, other=>panic!("unexpected {other:?}"),
                }
            }
            assert!(client.try_read().await.is_err());
            peer.await.unwrap();
        }
    }).await.unwrap();
}
