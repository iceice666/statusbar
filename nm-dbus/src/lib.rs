use zbus::proxy;
use zbus::{Connection, Result};

#[derive(Debug, Clone)]
pub struct AccessPoint {
    pub ssid: String,
    pub strength: u8,
    pub is_secured: bool,
    pub path: String,
}

#[derive(Debug, Clone)]
pub struct ActiveConnection {
    pub ssid: String,
    pub strength: u8,
    pub interface: String,
    pub ip_address: String,
}

// NetworkManager D-Bus proxy
#[proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait NetworkManager {
    /// Get list of devices
    fn get_devices(&self) -> Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// Get active connections
    #[zbus(property)]
    fn active_connections(&self) -> Result<Vec<zbus::zvariant::OwnedObjectPath>>;
}

// NetworkManager Device proxy
#[proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Device {
    /// Device interface name (e.g., wlan0)
    #[zbus(property)]
    fn interface(&self) -> Result<String>;

    /// Device type (2 = WiFi)
    #[zbus(property)]
    fn device_type(&self) -> Result<u32>;

    /// IP4Config path
    #[zbus(property)]
    fn ip4_config(&self) -> Result<zbus::zvariant::OwnedObjectPath>;

    /// Disconnect the device
    fn disconnect(&self) -> Result<()>;
}

// NetworkManager Wireless Device proxy
#[proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Wireless {
    /// Get list of access points
    fn get_access_points(&self) -> Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// Request scan
    fn request_scan(
        &self,
        options: std::collections::HashMap<String, zbus::zvariant::Value<'_>>,
    ) -> Result<()>;

    /// Active access point
    #[zbus(property)]
    fn active_access_point(&self) -> Result<zbus::zvariant::OwnedObjectPath>;
}

// AccessPoint proxy
#[proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
trait AccessPointProxy {
    /// SSID as byte array
    #[zbus(property)]
    fn ssid(&self) -> Result<Vec<u8>>;

    /// Signal strength (0-100)
    #[zbus(property)]
    fn strength(&self) -> Result<u8>;

    /// WPA flags (0 = open network)
    #[zbus(property, name = "WpaFlags")]
    fn wpa_flags(&self) -> Result<u32>;

    /// RSN flags (0 = open network)
    #[zbus(property, name = "RsnFlags")]
    fn rsn_flags(&self) -> Result<u32>;
}

// IP4Config proxy
#[proxy(
    interface = "org.freedesktop.NetworkManager.IP4Config",
    default_service = "org.freedesktop.NetworkManager"
)]
trait IP4Config {
    /// Address data
    #[zbus(property)]
    fn address_data(
        &self,
    ) -> Result<Vec<std::collections::HashMap<String, zbus::zvariant::OwnedValue>>>;
}

// ActiveConnection proxy
#[proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
trait ActiveConnectionProxy {
    /// Devices in this connection
    #[zbus(property)]
    fn devices(&self) -> Result<Vec<zbus::zvariant::OwnedObjectPath>>;

    /// Deactivate connection
    fn deactivate(&self) -> Result<()>;
}

pub struct NetworkManagerClient {
    connection: Connection,
}

impl NetworkManagerClient {
    pub async fn new() -> Result<Self> {
        let connection = Connection::system().await?;
        Ok(Self { connection })
    }

    /// Scan for available WiFi networks
    pub async fn scan_networks(&self) -> Result<Vec<AccessPoint>> {
        let nm_proxy = NetworkManagerProxy::new(&self.connection).await?;
        let devices = nm_proxy.get_devices().await?;

        let mut all_access_points = Vec::new();

        for device_path in devices {
            let device_proxy = DeviceProxy::builder(&self.connection)
                .path(&device_path)?
                .build()
                .await?;

            // Check if it's a WiFi device (type 2)
            if device_proxy.device_type().await? != 2 {
                continue;
            }

            let wireless_proxy = WirelessProxy::builder(&self.connection)
                .path(&device_path)?
                .build()
                .await?;

            // Request a scan
            let _ = wireless_proxy
                .request_scan(std::collections::HashMap::new())
                .await;

            // Get access points
            let ap_paths = wireless_proxy.get_access_points().await?;

            for ap_path in ap_paths {
                if let Ok(ap) = self.parse_access_point(&ap_path).await {
                    // Filter out duplicate SSIDs (keep strongest signal)
                    if let Some(existing) = all_access_points
                        .iter_mut()
                        .find(|a: &&mut AccessPoint| a.ssid == ap.ssid)
                    {
                        if ap.strength > existing.strength {
                            *existing = ap;
                        }
                    } else {
                        all_access_points.push(ap);
                    }
                }
            }
        }

        // Sort by signal strength (strongest first)
        all_access_points.sort_by(|a, b| b.strength.cmp(&a.strength));

        Ok(all_access_points)
    }

    /// Get currently active WiFi connection
    pub async fn get_active_connection(&self) -> Result<Option<ActiveConnection>> {
        let nm_proxy = NetworkManagerProxy::new(&self.connection).await?;
        let devices = nm_proxy.get_devices().await?;

        for device_path in devices {
            let device_proxy = DeviceProxy::builder(&self.connection)
                .path(&device_path)?
                .build()
                .await?;

            // Check if it's a WiFi device (type 2)
            if device_proxy.device_type().await? != 2 {
                continue;
            }

            let wireless_proxy = WirelessProxy::builder(&self.connection)
                .path(&device_path)?
                .build()
                .await?;

            let active_ap_path = wireless_proxy.active_access_point().await?;

            // Check if there's an active access point (not "/" means connected)
            if active_ap_path.as_str() == "/" {
                continue;
            }

            let interface = device_proxy.interface().await?;

            // Get access point info
            if let Ok(ap) = self.parse_access_point(&active_ap_path).await {
                // Get IP address
                let ip_address = self.get_ip_address(&device_proxy).await.unwrap_or_default();

                return Ok(Some(ActiveConnection {
                    ssid: ap.ssid,
                    strength: ap.strength,
                    interface,
                    ip_address,
                }));
            }
        }

        Ok(None)
    }

    /// Connect to a network (delegates to system authentication)
    pub async fn connect_to_network(&self, ssid: &str) -> Result<()> {
        // Use nmcli for connection (it handles system authentication)
        // This is simpler than implementing full D-Bus secret service integration
        tokio::process::Command::new("nmcli")
            .args(["device", "wifi", "connect", ssid])
            .output()
            .await
            .map_err(|e| zbus::Error::Failure(format!("Failed to connect: {}", e)))?;

        Ok(())
    }

    /// Disconnect from current network by interface name
    pub async fn disconnect(&self, interface: &str) -> Result<()> {
        let nm_proxy = NetworkManagerProxy::new(&self.connection).await?;
        let devices = nm_proxy.get_devices().await?;

        for device_path in devices {
            let device_proxy = DeviceProxy::builder(&self.connection)
                .path(&device_path)?
                .build()
                .await?;

            // Find the device with matching interface name
            if let Ok(device_interface) = device_proxy.interface().await {
                if device_interface == interface {
                    // Disconnect this device
                    return device_proxy.disconnect().await;
                }
            }
        }

        Err(zbus::Error::Failure(format!(
            "Device with interface '{}' not found",
            interface
        )))
    }

    // Helper: Parse access point information
    async fn parse_access_point(
        &self,
        ap_path: &zbus::zvariant::ObjectPath<'_>,
    ) -> Result<AccessPoint> {
        let ap_proxy = AccessPointProxyProxy::builder(&self.connection)
            .path(ap_path)?
            .build()
            .await?;

        let ssid_bytes = ap_proxy.ssid().await?;
        let ssid = String::from_utf8_lossy(&ssid_bytes).to_string();

        let strength = ap_proxy.strength().await?;

        let wpa_flags = ap_proxy.wpa_flags().await.unwrap_or(0);
        let rsn_flags = ap_proxy.rsn_flags().await.unwrap_or(0);
        let is_secured = wpa_flags != 0 || rsn_flags != 0;

        Ok(AccessPoint {
            ssid,
            strength,
            is_secured,
            path: ap_path.to_string(),
        })
    }

    // Helper: Get IP address from device
    async fn get_ip_address(&self, device_proxy: &DeviceProxy<'_>) -> Result<String> {
        let ip4_config_path = device_proxy.ip4_config().await?;

        if ip4_config_path.as_str() == "/" {
            return Ok(String::new());
        }

        let ip4_config_proxy = IP4ConfigProxy::builder(&self.connection)
            .path(&ip4_config_path)?
            .build()
            .await?;

        let address_data = ip4_config_proxy.address_data().await?;

        if let Some(first_addr) = address_data.first() {
            if let Some(addr_value) = first_addr.get("address") {
                if let Ok(addr_str) = addr_value.downcast_ref::<zbus::zvariant::Str>() {
                    return Ok(addr_str.to_string());
                }
            }
        }

        Ok(String::new())
    }
}
