use gtk::prelude::*;
use relm4::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::models::NetworkItem;
use nm_dbus::NetworkManagerClient;
use crate::widgets::popover::{PopoverComponent, PopoverInit, PopoverItem, PopoverMsg};

pub struct WiFi {
    ssid: String,
    signal_strength: i32,
    is_connected: bool,
    interface: String,
    ip_address: String,
    available_networks: Vec<NetworkItem>,
    is_scanning: bool,
    popover: Controller<PopoverComponent>,
    nm_client: Arc<Mutex<Option<NetworkManagerClient>>>,
    sender: Option<ComponentSender<Self>>,
}

#[derive(Debug, Clone)]
pub enum WiFiMsg {
    Update,
    TogglePopover,
    ScanNetworks,
    NetworksScanned(Vec<NetworkItem>),
    ConnectToNetwork(String),
    Disconnect,
    ConnectionResult(Result<(), String>),
}

#[relm4::component(pub)]
impl SimpleComponent for WiFi {
    type Init = ();
    type Input = WiFiMsg;
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_css_classes: &["wifi-widget", "widget"],

            #[name = "wifi_button"]
            gtk::Button {
                set_css_classes: &["wifi-button"],
                connect_clicked => WiFiMsg::TogglePopover,
                #[watch]
                set_visible: model.is_connected,

                gtk::Label {
                    #[watch]
                    set_label: &Self::signal_icon(model.signal_strength),
                    set_css_classes: &["wifi-icon"],
                    #[watch]
                    set_tooltip_text: Some(&model.ssid),
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Create popover without parent (will be set after widgets are created)
        let popover = PopoverComponent::builder()
            .launch(PopoverInit {
                parent: None,
                title: "WiFi".to_string(),
                margin: 12,
                spacing: 8,
                width: Some(320),
            })
            .detach();

        let model = WiFi {
            ssid: String::new(),
            signal_strength: 0,
            is_connected: false,
            interface: String::new(),
            ip_address: String::new(),
            available_networks: Vec::new(),
            is_scanning: false,
            popover,
            nm_client: Arc::new(Mutex::new(None)),
            sender: Some(sender.clone()),
        };

        let widgets = view_output!();

        // Set parent widget after widgets are created
        model
            .popover
            .model()
            .set_parent(&widgets.wifi_button.clone().upcast::<gtk::Widget>());

        // Initialize NetworkManager client asynchronously
        let nm_client = model.nm_client.clone();
        let sender_clone = sender.clone();
        tokio::spawn(async move {
            match NetworkManagerClient::new().await {
                Ok(client) => {
                    *nm_client.lock().await = Some(client);
                    sender_clone.input(WiFiMsg::Update);
                }
                Err(e) => {
                    eprintln!("Failed to initialize NetworkManager client: {}", e);
                }
            }
        });

        // Update every 10 seconds (reduced frequency since we have D-Bus signals)
        let sender_clone = sender.clone();
        gtk::glib::timeout_add_seconds_local(10, move || {
            sender_clone.input(WiFiMsg::Update);
            gtk::glib::ControlFlow::Continue
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            WiFiMsg::Update => {
                self.refresh_wifi_info(sender.clone());
            }
            WiFiMsg::TogglePopover => {
                self.popover.emit(PopoverMsg::Toggle);
                // Scan for networks when popover opens
                sender.input(WiFiMsg::ScanNetworks);
            }
            WiFiMsg::ScanNetworks => {
                if !self.is_scanning {
                    self.scan_networks(sender.clone());
                }
            }
            WiFiMsg::NetworksScanned(networks) => {
                self.is_scanning = false;
                self.available_networks = networks;
                self.update_popover_content();
            }
            WiFiMsg::ConnectToNetwork(ssid) => {
                self.connect_to_network(ssid, sender.clone());
            }
            WiFiMsg::Disconnect => {
                self.disconnect(sender.clone());
            }
            WiFiMsg::ConnectionResult(result) => {
                match result {
                    Ok(_) => {
                        // Refresh after successful connection
                        sender.input(WiFiMsg::Update);
                        sender.input(WiFiMsg::ScanNetworks);
                    }
                    Err(e) => {
                        eprintln!("Connection error: {}", e);
                    }
                }
            }
        }
    }
}

impl WiFi {
    fn refresh_wifi_info(&mut self, _sender: ComponentSender<Self>) {
        let nm_client = self.nm_client.clone();

        tokio::spawn(async move {
            if let Some(client) = nm_client.lock().await.as_ref() {
                match client.get_active_connection().await {
                    Ok(Some(_conn)) => {
                        // Will trigger update through message passing
                        // For now, we'll just log it
                    }
                    Ok(None) => {
                        // No active connection
                    }
                    Err(e) => {
                        eprintln!("Failed to get active connection: {}", e);
                    }
                }
            }
        });

        // Temporary: Still use nmcli for current connection info
        // This will be replaced with D-Bus signals in a future enhancement
        self.refresh_wifi_info_nmcli();
        self.update_popover_content();
    }

    fn refresh_wifi_info_nmcli(&mut self) {
        // Fallback to nmcli for current connection (temporary)
        if let Ok(output) = std::process::Command::new("nmcli")
            .args(["-t", "-f", "ACTIVE,SSID,SIGNAL,DEVICE", "dev", "wifi"])
            .output()
        {
            if let Ok(result) = String::from_utf8(output.stdout) {
                for line in result.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 4 && parts[0] == "yes" {
                        self.ssid = parts[1].to_string();
                        self.signal_strength = parts[2].parse().unwrap_or(0);
                        self.interface = parts[3].to_string();
                        self.is_connected = true;

                        // Get IP address
                        if let Ok(ip_output) = std::process::Command::new("ip")
                            .args(["-4", "addr", "show", &self.interface])
                            .output()
                        {
                            if let Ok(ip_result) = String::from_utf8(ip_output.stdout) {
                                for ip_line in ip_result.lines() {
                                    if ip_line.trim().starts_with("inet ") {
                                        if let Some(ip) = ip_line.trim().split_whitespace().nth(1) {
                                            self.ip_address = ip.to_string();
                                        }
                                    }
                                }
                            }
                        }
                        return;
                    }
                }
            }
        }

        // No active connection found
        self.is_connected = false;
        self.ssid = String::new();
        self.signal_strength = 0;
        self.interface = String::new();
        self.ip_address = String::new();
    }

    fn scan_networks(&mut self, sender: ComponentSender<Self>) {
        self.is_scanning = true;
        let nm_client = self.nm_client.clone();
        let current_ssid = self.ssid.clone();

        tokio::spawn(async move {
            if let Some(client) = nm_client.lock().await.as_ref() {
                match client.scan_networks().await {
                    Ok(access_points) => {
                        let networks: Vec<NetworkItem> = access_points
                            .iter()
                            .map(|ap| NetworkItem {
                                ssid: ap.ssid.clone(),
                                strength: ap.strength,
                                is_secured: ap.is_secured,
                                is_connected: ap.ssid == current_ssid,
                            })
                            .collect();

                        sender.input(WiFiMsg::NetworksScanned(networks));
                    }
                    Err(e) => {
                        eprintln!("Failed to scan networks: {}", e);
                        sender.input(WiFiMsg::NetworksScanned(Vec::new()));
                    }
                }
            }
        });
    }

    fn connect_to_network(&self, ssid: String, sender: ComponentSender<Self>) {
        let nm_client = self.nm_client.clone();

        tokio::spawn(async move {
            if let Some(client) = nm_client.lock().await.as_ref() {
                let result = client
                    .connect_to_network(&ssid)
                    .await
                    .map_err(|e| format!("Failed to connect: {}", e));

                sender.input(WiFiMsg::ConnectionResult(result));
            }
        });
    }

    fn disconnect(&self, sender: ComponentSender<Self>) {
        let nm_client = self.nm_client.clone();
        let interface = self.interface.clone();

        tokio::spawn(async move {
            if let Some(client) = nm_client.lock().await.as_ref() {
                let result = client
                    .disconnect(&interface)
                    .await
                    .map_err(|e| format!("Failed to disconnect: {}", e));

                sender.input(WiFiMsg::ConnectionResult(result));
            }
        });
    }

    fn update_popover_content(&self) {
        let mut items = Vec::new();

        // Current connection details
        if self.is_connected {
            items.push(PopoverItem::DetailRow {
                label: "Network".to_string(),
                value: self.ssid.clone(),
                value_css: "wifi-detail".to_string(),
            });
            items.push(PopoverItem::DetailRow {
                label: "Signal".to_string(),
                value: format!("{}%", self.signal_strength,),
                value_css: "wifi-detail".to_string(),
            });
            items.push(PopoverItem::DetailRow {
                label: "Interface".to_string(),
                value: self.interface.clone(),
                value_css: "wifi-detail".to_string(),
            });

            if !self.ip_address.is_empty() {
                items.push(PopoverItem::DetailRow {
                    label: "IP Address".to_string(),
                    value: self.ip_address.clone(),
                    value_css: "wifi-detail".to_string(),
                });
            }

            items.push(PopoverItem::Separator);
        }

        // Available networks section
        items.push(PopoverItem::Custom(self.create_networks_header()));

        if self.is_scanning {
            let loading_label = gtk::Label::new(Some("Scanning..."));
            loading_label.set_css_classes(&["network-loading"]);
            items.push(PopoverItem::Custom(loading_label.upcast::<gtk::Widget>()));
        } else if self.available_networks.is_empty() {
            let empty_label = gtk::Label::new(Some("No networks found"));
            empty_label.set_css_classes(&["network-empty"]);
            items.push(PopoverItem::Custom(empty_label.upcast::<gtk::Widget>()));
        } else {
            // Create scrollable network list
            let scrolled = gtk::ScrolledWindow::new();
            scrolled.set_min_content_height(300);
            scrolled.set_max_content_height(600);
            scrolled.set_vexpand(true);
            scrolled.set_hexpand(true);
            scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);

            let list_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
            list_box.set_css_classes(&["network-list"]);

            for network in &self.available_networks {
                list_box.append(&self.create_network_item(network));
            }

            scrolled.set_child(Some(&list_box));
            items.push(PopoverItem::Custom(scrolled.upcast::<gtk::Widget>()));
        }

        self.popover.emit(PopoverMsg::UpdateItems(items));
    }

    fn create_networks_header(&self) -> gtk::Widget {
        let header_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        header_box.set_css_classes(&["networks-header"]);

        let label = gtk::Label::new(Some("Available Networks"));
        label.set_halign(gtk::Align::Start);
        label.set_hexpand(true);
        label.set_css_classes(&["networks-title"]);

        let disconnect_btn = gtk::Button::with_label("X");
        disconnect_btn.set_css_classes(&["disconnect-button", "flat"]);
        disconnect_btn.set_tooltip_text(Some("Disconnect"));

        // Wire up disconnect button click handler
        if let Some(sender) = &self.sender {
            let sender_clone = sender.clone();
            disconnect_btn.connect_clicked(move |_| {
                sender_clone.input(WiFiMsg::Disconnect);
            });
        }

        let refresh_btn = gtk::Button::from_icon_name("view-refresh-symbolic");
        refresh_btn.set_css_classes(&["refresh-button", "flat"]);
        refresh_btn.set_tooltip_text(Some("Refresh networks"));

        // Wire up refresh button click handler
        if let Some(sender) = &self.sender {
            let sender_clone = sender.clone();
            refresh_btn.connect_clicked(move |_| {
                sender_clone.input(WiFiMsg::ScanNetworks);
            });
        }

        header_box.append(&label);
        header_box.append(&disconnect_btn);
        header_box.append(&refresh_btn);

        header_box.upcast::<gtk::Widget>()
    }

    fn create_network_item(&self, network: &NetworkItem) -> gtk::Widget {
        let button = gtk::Button::new();
        button.set_css_classes(&["network-item"]);

        let content_box = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        content_box.set_margin_start(8);
        content_box.set_margin_end(8);
        content_box.set_margin_top(6);
        content_box.set_margin_bottom(6);

        // Signal icon
        let signal_icon = gtk::Label::new(Some(network.signal_icon()));
        signal_icon.set_css_classes(&["network-signal"]);

        // SSID label
        let ssid_label = gtk::Label::new(Some(&network.ssid));
        ssid_label.set_halign(gtk::Align::Start);
        ssid_label.set_hexpand(true);
        ssid_label.set_css_classes(&["network-ssid"]);
        ssid_label.set_ellipsize(gtk::pango::EllipsizeMode::End);

        // Lock icon for secured networks
        if network.is_secured {
            let lock_icon = gtk::Label::new(Some(network.lock_icon()));
            lock_icon.set_css_classes(&["network-lock"]);
            content_box.append(&lock_icon);
        }

        // Connected badge
        if network.is_connected {
            let connected_label = gtk::Label::new(Some("Connected"));
            connected_label.set_css_classes(&["network-connected"]);
            content_box.append(&connected_label);
        }

        content_box.prepend(&ssid_label);
        content_box.prepend(&signal_icon);

        button.set_child(Some(&content_box));

        // Wire up network item click handler to connect
        if let Some(sender) = &self.sender {
            let sender_clone = sender.clone();
            let ssid = network.ssid.clone();
            button.connect_clicked(move |_| {
                sender_clone.input(WiFiMsg::ConnectToNetwork(ssid.clone()));
            });
        }

        button.upcast::<gtk::Widget>()
    }

    fn signal_icon(strength: i32) -> &'static str {
        match strength {
            80..=100 => "󰤨", // Full signal
            60..=79 => "󰤥",  // Good signal
            40..=59 => "󰤢",  // Medium signal
            20..=39 => "󰤟",  // Weak signal
            _ => "󰤯",        // No/very weak signal
        }
    }
}
