//! MPRIS DBus wrapper for media player control
//!
//! This crate provides a re-export of the mpris library functionality,
//! allowing the main statusbar application to have a consistent interface
//! for all DBus-related functionality.

// Re-export everything from mpris for convenience
pub use mpris::*;

// Type aliases for common usage patterns
pub type MediaPlayer = mpris::Player;
pub type MediaPlayerFinder = mpris::PlayerFinder;
pub type MediaMetadata = mpris::Metadata;
pub type MediaPlaybackStatus = mpris::PlaybackStatus;
