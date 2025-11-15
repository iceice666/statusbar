use gtk::prelude::*;
use relm4::prelude::*;
use std::fs;
use std::path::Path;

use super::popover::{PopoverComponent, PopoverInit, PopoverItem, PopoverMsg};

pub struct Battery {
    charge_level: f32,
    is_charging: bool,
    time_remaining: String,
    battery_path: Option<String>,
    popover: Controller<PopoverComponent>,
}

#[derive(Debug, Clone)]
pub enum BatteryMsg {
    Update,
    TogglePopover,
}

#[relm4::component(pub)]
impl SimpleComponent for Battery {
    type Init = ();
    type Input = BatteryMsg;
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_css_classes: &["battery-widget", "widget"],
            #[watch]
            set_visible: model.battery_path.is_some(),

            #[name = "battery_button"]
            gtk::Button {
                set_css_classes: &["battery-button"],
                connect_clicked => BatteryMsg::TogglePopover,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 4,

                    gtk::Label {
                        #[watch]
                        set_label: &Self::battery_icon(model.charge_level, model.is_charging),
                        #[watch]
                        set_css_classes: &["battery-icon", &Self::battery_status_class(model.charge_level, model.is_charging)],
                    },
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Find battery
        let battery_path = Self::find_battery();

        // Create popover without parent (will be set after widgets are created)
        let popover = PopoverComponent::builder()
            .launch(PopoverInit {
                parent: None,
                title: "Battery Details".to_string(),
                margin: 12,
                spacing: 8,
                width: None,
            })
            .detach();

        let model = Battery {
            charge_level: 0.0,
            is_charging: false,
            time_remaining: String::new(),
            battery_path,
            popover,
        };

        let widgets = view_output!();

        // Set parent widget after widgets are created
        model
            .popover
            .model()
            .set_parent(&widgets.battery_button.clone().upcast::<gtk::Widget>());

        // Update every 30 seconds
        let sender_clone = sender.clone();
        gtk::glib::timeout_add_seconds_local(30, move || {
            sender_clone.input(BatteryMsg::Update);
            gtk::glib::ControlFlow::Continue
        });

        // Initial update
        sender.input(BatteryMsg::Update);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            BatteryMsg::Update => {
                self.refresh_battery_info();
                self.update_popover_content();
            }
            BatteryMsg::TogglePopover => {
                self.popover.emit(PopoverMsg::Toggle);
            }
        }
    }
}

impl Battery {
    fn find_battery() -> Option<String> {
        let power_supply = Path::new("/sys/class/power_supply");

        if let Ok(entries) = fs::read_dir(power_supply) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str.starts_with("BAT") {
                        return Some(path.to_string_lossy().to_string());
                    }
                }
            }
        }
        None
    }

    fn refresh_battery_info(&mut self) {
        if let Some(ref battery_path) = self.battery_path {
            // Read capacity
            if let Ok(capacity) = fs::read_to_string(format!("{}/capacity", battery_path)) {
                self.charge_level = capacity.trim().parse().unwrap_or(0.0);
            }

            // Read status
            if let Ok(status) = fs::read_to_string(format!("{}/status", battery_path)) {
                self.is_charging = status.trim() == "Charging";
            }

            // Calculate time remaining (simplified)
            if let Ok(energy_now) = fs::read_to_string(format!("{}/energy_now", battery_path)) {
                if let Ok(power_now) = fs::read_to_string(format!("{}/power_now", battery_path)) {
                    if let (Ok(energy), Ok(power)) = (
                        energy_now.trim().parse::<f32>(),
                        power_now.trim().parse::<f32>(),
                    ) {
                        if power > 0.0 {
                            let hours = energy / power;
                            let h = hours as i32;
                            let m = ((hours - h as f32) * 60.0) as i32;

                            if self.is_charging {
                                self.time_remaining = format!("{}h {}m until full", h, m);
                            } else {
                                self.time_remaining = format!("{}h {}m remaining", h, m);
                            }
                        } else {
                            self.time_remaining = "Calculating...".to_string();
                        }
                    }
                }
            }
        }
    }

    fn update_popover_content(&self) {
        let mut items = vec![
            PopoverItem::DetailRow {
                label: "Battery Level".to_string(),
                value: format!("{}%", self.charge_level as i32),
                value_css: "battery-detail".to_string(),
            },
            PopoverItem::DetailRow {
                label: "Status".to_string(),
                value: if self.is_charging {
                    "Charging".to_string()
                } else {
                    "Discharging".to_string()
                },
                value_css: "battery-detail".to_string(),
            },
        ];

        // Add time remaining if available
        if !self.time_remaining.is_empty() && self.time_remaining != "Calculating..." {
            items.push(PopoverItem::DetailRow {
                label: "Time".to_string(),
                value: self.time_remaining.clone(),
                value_css: "battery-detail".to_string(),
            });
        }

        self.popover.emit(PopoverMsg::UpdateItems(items));
    }

    fn battery_icon(level: f32, charging: bool) -> &'static str {
        if charging {
            return "󰂄"; // Charging icon
        }

        match level as i32 {
            90..=100 => "󰁹", // Full
            80..=89 => "󰂂",  // 90%
            70..=79 => "󰂁",  // 80%
            60..=69 => "󰂀",  // 70%
            50..=59 => "󰁿",  // 60%
            40..=49 => "󰁾",  // 50%
            30..=39 => "󰁽",  // 40%
            20..=29 => "󰁼",  // 30%
            10..=19 => "󰁻",  // 20%
            _ => "󰁺",        // 10% or less
        }
    }

    fn battery_status_class(level: f32, charging: bool) -> String {
        if charging {
            "battery-charging".to_string()
        } else if level <= 20.0 {
            "battery-low".to_string()
        } else if level <= 50.0 {
            "battery-medium".to_string()
        } else {
            "battery-full".to_string()
        }
    }
}
