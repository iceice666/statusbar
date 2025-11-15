use std::error::Error;

use gtk::prelude::*;
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use relm4::prelude::*;

mod widgets;
use widgets::{
    Battery, Clock, MediaPlayer, Resources, SystemTray, WiFi, WindowTitle, WorkspaceWidget,
};

const APP_ID: &str = "com.github.iceice666.statusbar";

struct StatusBar {
    workspace: Controller<WorkspaceWidget>,
    window_title: Controller<WindowTitle>,
    media_player: Controller<MediaPlayer>,
    resources: Controller<Resources>,
    wifi: Controller<WiFi>,
    battery: Controller<Battery>,
    tray: Controller<SystemTray>,
    clock: Controller<Clock>,
}

#[derive(Debug)]
enum StatusBarMsg {
    // Messages will go here
}

#[relm4::component]
impl SimpleComponent for StatusBar {
    type Init = ();
    type Input = StatusBarMsg;
    type Output = ();

    view! {
        #[root]
        #[name = "window"]
        gtk::ApplicationWindow {
            set_css_classes: &["statusbar-window"],
            set_height_request: 32,
            set_default_height: 32,

            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                set_spacing: 0,
                set_css_classes: &["statusbar-container"],

                // Left section
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 2,
                    set_hexpand: true,
                    set_halign: gtk::Align::Start,
                    set_css_classes: &["statusbar-left"],

                    #[local_ref]
                    workspace_widget -> gtk::Box {},

                    #[local_ref]
                    window_title_widget -> gtk::Box {},

                    #[local_ref]
                    media_player_widget -> gtk::Box {},
                },

                // Right section
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_spacing: 2,
                    set_hexpand: true,
                    set_halign: gtk::Align::End,
                    set_css_classes: &["statusbar-right"],

                    #[local_ref]
                    resources_widget -> gtk::Box {},

                    #[local_ref]
                    wifi_widget -> gtk::Box {},

                    #[local_ref]
                    battery_widget -> gtk::Box {},

                    #[local_ref]
                    tray_widget -> gtk::Box {},

                    #[local_ref]
                    clock_widget -> gtk::Box {},
                }
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // Initialize layer shell BEFORE window is realized
        root.init_layer_shell();

        // Configure layer shell properties
        root.set_layer(Layer::Overlay);
        root.set_namespace(Some("statusbar"));
        root.auto_exclusive_zone_enable();

        // Anchor to top, left, and right edges (spans full width)
        root.set_anchor(Edge::Top, true);
        root.set_anchor(Edge::Left, true);
        root.set_anchor(Edge::Right, true);
        root.set_anchor(Edge::Bottom, false);

        // Set margins (0 for now)
        root.set_margin(Edge::Top, 0);
        root.set_margin(Edge::Bottom, 0);
        root.set_margin(Edge::Left, 0);
        root.set_margin(Edge::Right, 0);

        // Initialize widgets
        let workspace = WorkspaceWidget::builder().launch(()).detach();
        let window_title = WindowTitle::builder().launch(()).detach();
        let media_player = MediaPlayer::builder().launch(()).detach();
        let resources = Resources::builder().launch(()).detach();
        let wifi = WiFi::builder().launch(()).detach();
        let battery = Battery::builder().launch(()).detach();
        let tray = SystemTray::builder().launch(()).detach();
        let clock = Clock::builder().launch(()).detach();

        let model = StatusBar {
            workspace,
            window_title,
            media_player,
            resources,
            wifi,
            battery,
            tray,
            clock,
        };

        let workspace_widget = model.workspace.widget();
        let window_title_widget = model.window_title.widget();
        let media_player_widget = model.media_player.widget();
        let resources_widget = model.resources.widget();
        let wifi_widget = model.wifi.widget();
        let battery_widget = model.battery.widget();
        let tray_widget = model.tray.widget();
        let clock_widget = model.clock.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, _msg: Self::Input, _sender: ComponentSender<Self>) {
        // Message handling will go here
    }
}

/// Compile SCSS to CSS at runtime
fn compile_scss() -> Result<String, String> {
    let scss_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("theme")
        .join("style.scss");

    grass::from_path(&scss_path, &grass::Options::default())
        .map_err(|e| format!("Failed to compile SCSS:\n{}", e))
}

fn main() -> Result<(), Box<dyn Error>> {
    gtk::init()?;

    // Compile SCSS to CSS at runtime
    let css = match compile_scss() {
        Ok(css) => css,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    #[cfg(debug_assertions)]
    std::fs::write("./final.css", &css)?;

    // Load compiled CSS
    let css_provider = gtk::CssProvider::new();
    css_provider.load_from_data(&css);

    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("Could not connect to display"),
        &css_provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let app = RelmApp::new(APP_ID);
    app.run::<StatusBar>(());

    Ok(())
}
