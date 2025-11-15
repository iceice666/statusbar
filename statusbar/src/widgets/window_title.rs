use gtk::prelude::*;
use niri_ipc::{Event, Request, Response, socket::Socket};
use relm4::prelude::*;
use std::thread;

pub struct WindowTitle {
    title: String,
    app_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum WindowTitleMsg {
    UpdateTitle(String, Option<String>),
}

#[relm4::component(pub)]
impl SimpleComponent for WindowTitle {
    type Init = ();
    type Input = WindowTitleMsg;
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 8,
            set_css_classes: &["window-title-widget", "widget"],

            gtk::Label {
                #[watch]
                set_label: &model.title,
                set_css_classes: &["window-title-label"],
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                set_max_width_chars: 50,
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = WindowTitle {
            title: String::from(""),
            app_id: None,
        };

        let widgets = view_output!();

        // Spawn thread to listen for niri events
        let sender_clone = sender.clone();
        thread::spawn(move || {
            if let Err(e) = Self::listen_niri_events(sender_clone) {
                eprintln!("Niri IPC error: {}", e);
            }
        });

        // Request initial focused window
        thread::spawn(move || {
            if let Some((title, app_id)) = Self::get_focused_window() {
                sender.input(WindowTitleMsg::UpdateTitle(title, app_id));
            }
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            WindowTitleMsg::UpdateTitle(title, app_id) => {
                self.title = title;
                self.app_id = app_id;
            }
        }
    }
}

impl WindowTitle {
    fn listen_niri_events(sender: ComponentSender<Self>) -> Result<(), String> {
        let mut socket = Socket::connect().map_err(|e| e.to_string())?;

        let reply = socket
            .send(Request::EventStream)
            .map_err(|e| e.to_string())?;

        if matches!(reply, Ok(Response::Handled)) {
            let mut read_event = socket.read_events();

            while let Ok(event) = read_event() {
                match event {
                    Event::WindowFocusChanged { id: _ } => {
                        if let Some((title, app_id)) = Self::get_focused_window() {
                            sender.input(WindowTitleMsg::UpdateTitle(title, app_id));
                        }
                    }
                    Event::WindowsChanged { windows: _ } => {
                        if let Some((title, app_id)) = Self::get_focused_window() {
                            sender.input(WindowTitleMsg::UpdateTitle(title, app_id));
                        }
                    }
                    Event::WindowOpenedOrChanged { window } => {
                        if window.is_focused {
                            sender.input(WindowTitleMsg::UpdateTitle(
                                window.title.clone().unwrap_or_default(),
                                window.app_id.clone(),
                            ));
                        }
                    }
                    Event::WindowClosed { id: _ } => {
                        if let Some((title, app_id)) = Self::get_focused_window() {
                            sender.input(WindowTitleMsg::UpdateTitle(title, app_id));
                        } else {
                            sender.input(WindowTitleMsg::UpdateTitle(String::new(), None));
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn get_focused_window() -> Option<(String, Option<String>)> {
        let mut socket = Socket::connect().ok()?;
        let reply = socket.send(Request::FocusedWindow).ok()?;

        match reply {
            Ok(Response::FocusedWindow(Some(window))) => {
                Some((window.title.unwrap_or_default(), window.app_id))
            }
            _ => None,
        }
    }
}
