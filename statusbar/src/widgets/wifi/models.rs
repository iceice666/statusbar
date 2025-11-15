/// Represents a WiFi network in the available networks list
#[derive(Debug, Clone)]
pub struct NetworkItem {
    pub ssid: String,
    pub strength: u8,
    pub is_secured: bool,
    pub is_connected: bool,
}

impl NetworkItem {
    pub fn signal_icon(&self) -> &'static str {
        match self.strength {
            80..=100 => "󰤨", // Full signal
            60..=79 => "󰤥",  // Good signal
            40..=59 => "󰤢",  // Medium signal
            20..=39 => "󰤟",  // Weak signal
            _ => "󰤯",        // No/very weak signal
        }
    }

    pub fn lock_icon(&self) -> &'static str {
        if self.is_secured {
            "󰌾" // Lock icon for secured networks
        } else {
            "" // No icon for open networks
        }
    }
}

/// Connection information returned from async operations
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub ssid: String,
    pub signal_strength: i32,
    pub interface: String,
    pub ip_address: String,
}

/// Status of network scanning operations
#[derive(Debug, Clone)]
pub enum ScanStatus {
    Idle,
    Scanning,
    Complete,
    Failed(String),
}
