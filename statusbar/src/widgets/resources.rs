use gtk::prelude::*;
use relm4::prelude::*;
use sysinfo::{Components, Disks, Networks, System};

use super::popover::{PopoverComponent, PopoverInit, PopoverItem, PopoverMsg};

pub struct Resources {
    cpu_usage: f32,
    cpu_label: String,
    cpu_label_widget: gtk::Label,
    memory_used: u64,
    memory_total: u64,
    memory_label: String,
    memory_label_widget: gtk::Label,
    network_rx: u64,
    network_tx: u64,
    disk_read: u64,
    disk_write: u64,
    gpu_usage: f32,
    temperatures: Vec<(String, f32)>,
    system: System,
    networks: Networks,
    components: Components,
    disks: Disks,
    popover: Controller<PopoverComponent>,
}

#[derive(Debug, Clone)]
pub enum ResourcesMsg {
    Update,
    TogglePopover,
}

#[relm4::component(pub)]
impl SimpleComponent for Resources {
    type Init = ();
    type Input = ResourcesMsg;
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_css_classes: &["resources-widget", "widget"],

            #[name = "resources_button"]
            gtk::Button {
                set_css_classes: &["resources-button"],
                connect_clicked => ResourcesMsg::TogglePopover,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 8,

                    // CPU monitor (always visible)
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,
                        set_css_classes: &["resource-item"],

                        gtk::Label {
                            set_label: "CPU",
                            set_css_classes: &["resource-label"],
                        },

                        #[name(cpu_label_widget)]
                        gtk::Label {
                            set_label: "0%",
                            set_css_classes: &["resource-value", "cpu-value"],
                        }
                    },

                    // Memory monitor (always visible)
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_spacing: 0,
                        set_css_classes: &["resource-item"],

                        gtk::Label {
                            set_label: "MEM",
                            set_css_classes: &["resource-label"],
                        },

                        #[name(memory_label_widget)]
                        gtk::Label {
                            set_label: "0.0G",
                            set_css_classes: &["resource-value", "memory-value"],
                        }
                    }
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut system = System::new_all();
        system.refresh_all();

        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();
        let disks = Disks::new_with_refreshed_list();

        let widgets = view_output!();

        // Initialize the popover component
        let popover = PopoverComponent::builder()
            .launch(PopoverInit {
                parent: Some(widgets.resources_button.clone().upcast::<gtk::Widget>()),
                title: "System Resources".to_string(),
                margin: 16,
                spacing: 12,
                width: Some(350),
            })
            .detach();

        let model = Resources {
            cpu_usage: 0.0,
            cpu_label: "0%".to_string(),
            cpu_label_widget: widgets.cpu_label_widget.clone(),
            memory_used: 0,
            memory_total: system.total_memory(),
            memory_label: "0.0G (0%)".to_string(),
            memory_label_widget: widgets.memory_label_widget.clone(),
            network_rx: 0,
            network_tx: 0,
            disk_read: 0,
            disk_write: 0,
            gpu_usage: 0.0,
            temperatures: Vec::new(),
            system,
            networks,
            components,
            disks,
            popover,
        };

        // Update every 2 seconds
        let sender_clone = sender.clone();
        gtk::glib::timeout_add_seconds_local(2, move || {
            sender_clone.input(ResourcesMsg::Update);
            gtk::glib::ControlFlow::Continue
        });

        // Initial update
        sender.input(ResourcesMsg::Update);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ResourcesMsg::Update => {
                self.refresh_stats();
                self.update_popover_content();

                // Manually update the labels
                self.cpu_label_widget.set_label(&self.cpu_label);
                self.memory_label_widget.set_label(&self.memory_label);
            }
            ResourcesMsg::TogglePopover => {
                self.popover.emit(PopoverMsg::Toggle);
            }
        }
    }
}

impl Resources {
    fn refresh_stats(&mut self) {
        // Refresh CPU and memory
        self.system.refresh_cpu_all();
        self.system.refresh_memory();

        // Calculate average CPU usage
        self.cpu_usage = self.system.global_cpu_usage();
        self.cpu_label = format!("{:.0}%", self.cpu_usage);

        // Get memory usage
        self.memory_used = self.system.used_memory();
        self.memory_total = self.system.total_memory();
        self.memory_label = Self::format_memory_compact(self.memory_used);

        // Refresh network stats
        self.networks.refresh(false);

        // Calculate total network traffic
        let mut total_rx = 0;
        let mut total_tx = 0;

        for (_interface_name, network) in self.networks.iter() {
            total_rx += network.received();
            total_tx += network.transmitted();
        }

        self.network_rx = total_rx;
        self.network_tx = total_tx;

        // Refresh disk stats
        self.disks.refresh(true);

        // Note: sysinfo doesn't directly provide disk I/O rates, these would be cumulative
        // For now, we'll show placeholder values
        self.disk_read = 0;
        self.disk_write = 0;

        // Refresh temperature sensors
        self.components.refresh(true);
        self.temperatures.clear();
        for component in self.components.iter() {
            if let Some(temp) = component.temperature() {
                self.temperatures
                    .push((component.label().to_string(), temp));
            }
        }

        // GPU usage would require external tools/libraries
        self.gpu_usage = 0.0;
    }

    fn update_popover_content(&self) {
        let mut items = Vec::new();

        // CPU details
        items.push(PopoverItem::DetailRow {
            label: "CPU Usage".to_string(),
            value: format!("{:.1}%", self.cpu_usage),
            value_css: "cpu-detail".to_string(),
        });

        // Memory details
        let mem_percent = (self.memory_used as f64 / self.memory_total as f64) * 100.0;
        items.push(PopoverItem::DetailRow {
            label: "Memory".to_string(),
            value: format!(
                "{:.2}G / {:.2}G ({:.0}%)",
                self.memory_used as f64 / 1_073_741_824.0,
                self.memory_total as f64 / 1_073_741_824.0,
                mem_percent
            ),
            value_css: "memory-detail".to_string(),
        });

        // Network details
        items.push(PopoverItem::DetailRow {
            label: "Network RX".to_string(),
            value: Self::format_bytes(self.network_rx),
            value_css: "network-detail".to_string(),
        });
        items.push(PopoverItem::DetailRow {
            label: "Network TX".to_string(),
            value: Self::format_bytes(self.network_tx),
            value_css: "network-detail".to_string(),
        });

        // Disk usage
        for disk in self.disks.iter() {
            let disk_name = disk.name().to_string_lossy();
            let available = disk.available_space();
            let total = disk.total_space();
            let used_percent = ((total - available) as f64 / total as f64) * 100.0;
            items.push(PopoverItem::DetailRow {
                label: format!("Disk ({})", disk_name),
                value: format!(
                    "{:.1}G / {:.1}G ({:.0}%)",
                    (total - available) as f64 / 1_073_741_824.0,
                    total as f64 / 1_073_741_824.0,
                    used_percent
                ),
                value_css: "disk-detail".to_string(),
            });
        }

        // Temperature sensors
        if !self.temperatures.is_empty() {
            items.push(PopoverItem::Separator);
            for (label, temp) in &self.temperatures {
                items.push(PopoverItem::DetailRow {
                    label: label.clone(),
                    value: format!("{:.1}°C", temp),
                    value_css: "temp-detail".to_string(),
                });
            }
        }

        // GPU placeholder
        if self.gpu_usage > 0.0 {
            items.push(PopoverItem::Separator);
            items.push(PopoverItem::DetailRow {
                label: "GPU Usage".to_string(),
                value: format!("{:.1}%", self.gpu_usage),
                value_css: "gpu-detail".to_string(),
            });
        }

        // Update the popover with new items
        self.popover.emit(PopoverMsg::UpdateItems(items));
    }

    fn format_memory_short(used: u64, total: u64) -> String {
        let used_gb = used as f64 / 1_073_741_824.0;
        let percent = (used as f64 / total as f64) * 100.0;
        format!("{:.1}G ({:.0}%)", used_gb, percent)
    }

    fn format_memory_compact(used: u64) -> String {
        let used_gb = used as f64 / 1_073_741_824.0;
        format!("{:.1}G", used_gb)
    }

    fn format_bytes(bytes: u64) -> String {
        const KB: u64 = 1_024;
        const MB: u64 = 1_048_576;
        const GB: u64 = 1_073_741_824;

        if bytes >= GB {
            format!("{:.1}G", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.1}M", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.0}K", bytes as f64 / KB as f64)
        } else {
            format!("{}B", bytes)
        }
    }
}
