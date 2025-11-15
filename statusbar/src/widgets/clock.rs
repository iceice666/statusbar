use chrono::Local;
use gtk::glib;
use gtk::prelude::*;
use relm4::prelude::*;

pub struct Clock {
    current_time: String,
    popover: gtk::Popover,
}

#[derive(Debug)]
pub enum ClockMsg {
    UpdateTime,
    ToggleCalendar,
}

#[relm4::component(pub)]
impl SimpleComponent for Clock {
    type Init = ();
    type Input = ClockMsg;
    type Output = ();

    view! {
        #[name = "clock_box"]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 0,
            set_css_classes: &["clock-widget", "widget"],

            #[name = "time_button"]
            gtk::Button {
                set_css_classes: &["clock-time-button"],

                gtk::Label {
                    #[watch]
                    set_label: &model.current_time,
                    set_css_classes: &["clock-label"],
                },

                connect_clicked => ClockMsg::ToggleCalendar,
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Create calendar popover
        let calendar = gtk::Calendar::new();
        let popover = gtk::Popover::builder().child(&calendar).build();

        let model = Clock {
            current_time: Self::format_time(),
            popover: popover.clone(),
        };

        let widgets = view_output!();

        // Set popover parent to the time button
        popover.set_parent(&widgets.time_button);

        // Set up periodic updates (every second)
        glib::timeout_add_seconds_local(1, move || {
            sender.input(ClockMsg::UpdateTime);
            glib::ControlFlow::Continue
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            ClockMsg::UpdateTime => {
                self.current_time = Self::format_time();
            }
            ClockMsg::ToggleCalendar => {
                if self.popover.is_visible() {
                    self.popover.popdown();
                } else {
                    self.popover.popup();
                }
            }
        }
    }
}

impl Clock {
    fn format_time() -> String {
        Local::now().format("%H:%M:%S").to_string()
    }
}
