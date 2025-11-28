use mdns_sd::{Error as mdns_error, IfKind, Receiver, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::{
    collections::HashMap,
    fmt,
    net::{IpAddr, SocketAddr},
    time::Duration,
};
use tokio::{sync::mpsc, task::JoinHandle};

const SERVICE_NAME: &str = "_esphomelib._tcp.local.";

/// Information about a discovered ESPHome device.
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    record: ServiceInfo,
}

impl Eq for DeviceInfo {}
impl PartialEq for DeviceInfo {
    fn eq(&self, other: &Self) -> bool {
        self.record.get_fullname() == other.record.get_fullname()
            && self.record.get_addresses() == other.record.get_addresses()
            && self.record.get_port() == other.record.get_port()
    }
}

impl DeviceInfo {
    /// Gets the device's socket address.
    #[must_use]
    pub fn socket_address(&self) -> Option<SocketAddr> {
        let addr = self.record.get_addresses().iter().next()?.to_owned();
        Some(SocketAddr::new(addr, self.record.get_port()))
    }

    /// Gets the device's hostname.
    #[must_use]
    pub fn hostname(&self) -> &str {
        self.record.get_hostname()
    }

    /// Gets all the device's attributes as a `HashMap`.
    #[must_use]
    pub fn attributes(&self) -> HashMap<String, String> {
        self.record.get_properties().clone().into_property_map_str()
    }

    /// Check if the device has the `api_encryption` attribute.
    /// This indicates that the device requires a key for communication.
    #[must_use]
    pub fn has_encryption(&self) -> bool {
        self.record.get_property("api_encryption").is_some()
    }
}

pub use crate::error::DiscoveryError as Error;

/// Client for discovering ESPHome devices using mDNS.
/// This client allows you to discover devices on the local network
/// that are running ESPHome and have the mDNS service enabled.
///
/// Requires the `discovery` feature to be enabled.
///
/// # Usage:
/// ```rust,no_run
/// use esphome_client::discovery::Client;
///
/// #[tokio::main]
/// async fn main() {
///     let mut discovery_results = Client::default().discover().expect("Results");
///     while let Ok(device) = discovery_results.next().await {   
///         println!("Discovered device: {device:?}");
///     }
/// }
/// ```
#[derive(Default, Debug)]
pub struct Client {
    interval: Option<Duration>,
    interface: Option<IfKind>,
    service_name: Option<String>,
}

impl Client {
    /// Set the interface ip to discover devices on.
    /// If not set, it will use the all interfaces.
    #[must_use]
    pub fn with_interface_ip(mut self, addr: IpAddr) -> Self {
        self.interface = Some(IfKind::Addr(addr));
        self
    }

    /// Set the interface name to discover devices on.
    /// If not set, it will use the all interfaces.
    #[must_use]
    pub fn with_interface(mut self, interface: &str) -> Self {
        self.interface = Some(IfKind::Name(interface.to_owned()));
        self
    }

    /// Set the service name to filter devices by.
    /// If not set, it will default to `_esphomelib._tcp.local`.
    #[must_use]
    pub fn with_service_name(mut self, service_name: impl Into<String>) -> Self {
        self.service_name = Some(service_name.into());
        self
    }

    /// Set custom ip check interval.
    /// Defaults to the default interval of the `mdns-sd` crate.
    #[must_use]
    pub const fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = Some(interval);
        self
    }

    /// Initialize the discovery client and start discovering devices.
    ///
    /// # Errors
    ///
    /// Will return `Error` if discovery cannot be started due to initialization issues,
    pub fn discover(self) -> Result<ResultStream, Error> {
        let service_name = self
            .service_name
            .as_deref()
            .unwrap_or(SERVICE_NAME)
            .to_owned();

        let mdns = ServiceDaemon::new().map_err(|e| Error::InitializationError {
            reason: e.to_string(),
        })?;
        if let Some(interval) = self.interval {
            if let Ok(interval) = interval.as_secs().try_into() {
                mdns.set_ip_check_interval(interval)
                    .map_err(|e| Error::InitializationError {
                        reason: e.to_string(),
                    })?;
            }
        }
        if let Some(interface) = self.interface {
            mdns.enable_interface(interface)
                .map_err(|e| Error::InitializationError {
                    reason: e.to_string(),
                })?;
        }
        let receiver = mdns
            .browse(&service_name)
            .map_err(|e| Error::InitializationError {
                reason: e.to_string(),
            })?;

        Ok(ResultStream::new(mdns, receiver))
    }
}

/// Stream of discovered devices.
/// This stream will yield `DeviceInfo` for each discovered device.
/// Note that this stream will not yield unique devices, so you may receive the same device multiple times.
pub struct ResultStream {
    mdns: ServiceDaemon,
    handle: JoinHandle<()>,
    rx: mpsc::Receiver<DeviceInfo>,
}

impl fmt::Debug for ResultStream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Results")
            .field("mdns", &"ServiceDaemon {}")
            .field("handle", &self.handle)
            .field("rx", &self.rx)
            .finish()
    }
}

impl ResultStream {
    fn new(mdns: ServiceDaemon, receiver: Receiver<ServiceEvent>) -> Self {
        let (tx, rx) = mpsc::channel(100);
        let handle = tokio::spawn(async move {
            while let Ok(event) = receiver.recv_async().await {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        tracing::debug!("Discovered device: {info:?}");
                        if let Err(e) = tx.send(DeviceInfo { record: info }).await {
                            tracing::error!("Failed to send discovered device info: {e}");
                        }
                    }
                    evt => tracing::debug!("Unhandled discovery event: {evt:?}"),
                }
            }
        });
        Self { mdns, handle, rx }
    }

    /// Get the next discovered device.
    ///
    /// Note that this will not return unique devices, so you may receive the same device multiple times.
    ///
    /// # Errors
    ///
    /// Will return `Error::Aborted` if the discovery was aborted.
    pub async fn next(&mut self) -> Result<DeviceInfo, Error> {
        self.rx.recv().await.ok_or(Error::Aborted)
    }

    /// Get the first discovered device.
    ///
    /// # Errors
    ///
    /// Will return `Error::Aborted` if the discovery was aborted.
    pub async fn first(mut self) -> Result<DeviceInfo, Error> {
        self.next().await
    }
}

impl Drop for ResultStream {
    fn drop(&mut self) {
        self.handle.abort();
        for _ in 0..5 {
            if matches!(self.mdns.shutdown(), Err(mdns_error::Again)) {
                // retry shutdown a few times
            } else {
                return;
            }
        }
        tracing::error!("Failed to shutdown mDNS daemon after multiple attempts");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::net::{IpAddr, Ipv4Addr};
    use std::time::Duration;

    #[test]
    fn test_device_info_hostname_and_attributes() {
        // Mock ServiceInfo
        let mut props: HashMap<String, String> = HashMap::new();
        props.insert("api_encryption".into(), "true".into());
        props.insert("foo".into(), "bar".into());
        let info = ServiceInfo::new(
            "_esphomelib._tcp.local",
            "test-device",
            "test.local",
            "127.0.0.1",
            6053,
            props,
        )
        .unwrap();

        let device = DeviceInfo { record: info };

        assert_eq!(device.hostname(), "test.local");
        let attrs = device.attributes();
        assert_eq!(attrs.get("foo"), Some(&"bar".to_owned()));
        assert!(device.has_encryption());
    }

    #[test]
    fn test_device_info_socket_address() {
        let info = ServiceInfo::new(
            "_esphomelib._tcp.local",
            "test-device",
            "test.local",
            "192.168.1.10",
            6053,
            HashMap::<String, String>::new(),
        )
        .unwrap();

        let device = DeviceInfo { record: info };
        let addr = device.socket_address().unwrap();
        assert_eq!(addr.ip(), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)));
        assert_eq!(addr.port(), 6053);
    }

    #[test]
    fn test_client_builder_methods() {
        let client = Client::default()
            .with_interface_ip(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))
            .with_interface("eth0")
            .with_service_name("_custom._tcp.local")
            .with_interval(Duration::from_secs(10));

        assert!(client.interval.is_some());
        assert!(client.interface.is_some());
        assert_eq!(client.service_name.as_deref(), Some("_custom._tcp.local"));
    }

    #[test]
    fn test_error_display() {
        let init_err = Error::InitializationError {
            reason: "fail".to_owned(),
        };
        let abort_err = Error::Aborted;
        assert_eq!(format!("{init_err}"), "Initialization error: fail");
        assert_eq!(format!("{abort_err}"), "Discovery aborted");
    }
}
