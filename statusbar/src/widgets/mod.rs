// Widget modules
pub mod clock;
pub mod workspace;
pub mod window_title;
pub mod media_player;
pub mod resources;
pub mod wifi;
pub mod battery;
pub mod tray;

// Popover component module
pub mod popover;

// Re-exports
pub use clock::Clock;
pub use workspace::WorkspaceWidget;
pub use window_title::WindowTitle;
pub use media_player::MediaPlayer;
pub use resources::Resources;
pub use wifi::WiFi;
pub use battery::Battery;
pub use tray::SystemTray;
