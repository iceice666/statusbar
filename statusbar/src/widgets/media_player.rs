use gtk::prelude::*;
use mpris_dbus::{PlaybackStatus, Player, PlayerFinder};
use relm4::{gtk::Orientation, prelude::*};

use super::popover::{PopoverComponent, PopoverInit, PopoverItem, PopoverMsg};

pub struct MediaPlayer {
    track_title: String,
    track_artist: String,
    track_album: String,
    is_playing: bool,
    player: Option<Player>,
    popover: Controller<PopoverComponent>,
}

#[derive(Debug, Clone)]
pub enum MediaPlayerMsg {
    UpdateTrack(String, String),
    UpdatePlaybackStatus(bool),
    PlayPause,
    Next,
    Previous,
    Refresh,
    TogglePopover,
}

#[relm4::component(pub)]
impl SimpleComponent for MediaPlayer {
    type Init = ();
    type Input = MediaPlayerMsg;
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: Orientation::Horizontal,
            set_spacing: 8,
            set_css_classes: &["media-player-widget", "widget"],
            #[watch]
            set_visible: !model.track_title.is_empty(),

            // Track info button (clickable)
            #[name = "track_button"]
            gtk::Button {
                set_css_classes: &["media-info-button"],
                connect_clicked => MediaPlayerMsg::TogglePopover,

                gtk::Label {
                    #[watch]
                    set_label: &model.track_title,
                    set_css_classes: &["media-title"],
                    set_ellipsize: gtk::pango::EllipsizeMode::End,
                    set_max_width_chars: 30,
                }
            },

            // Controls
            gtk::Box {
                set_orientation: Orientation::Horizontal,
                set_css_classes: &["media-controls"],

                gtk::Button {
                    set_label: "⏮",
                    set_css_classes: &["media-button"],
                    connect_clicked => MediaPlayerMsg::Previous,
                },

                gtk::Button {
                    #[watch]
                    set_label: if model.is_playing { "⏸" } else { "⏵" },
                    set_css_classes: &["media-button", "media-play-pause"],
                    connect_clicked => MediaPlayerMsg::PlayPause,
                },

                gtk::Button {
                    set_label: "⏭",
                    set_css_classes: &["media-button"],
                    connect_clicked => MediaPlayerMsg::Next,
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
                title: "Now Playing".to_string(),
                margin: 12,
                spacing: 8,
                width: Some(300),
            })
            .detach();

        let model = MediaPlayer {
            track_title: String::new(),
            track_artist: String::new(),
            track_album: String::new(),
            is_playing: false,
            player: None,
            popover,
        };

        let widgets = view_output!();

        // Set parent widget after widgets are created
        model
            .popover
            .model()
            .set_parent(&widgets.track_button.clone().upcast::<gtk::Widget>());

        // Periodic refresh to detect player changes and track updates
        let sender_clone = sender.clone();
        gtk::glib::timeout_add_seconds_local(2, move || {
            sender_clone.input(MediaPlayerMsg::Refresh);
            gtk::glib::ControlFlow::Continue
        });

        // Initial refresh
        sender.input(MediaPlayerMsg::Refresh);

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, _sender: ComponentSender<Self>) {
        match msg {
            MediaPlayerMsg::UpdateTrack(title, artist) => {
                self.track_title = title;
                self.track_artist = artist;
            }
            MediaPlayerMsg::UpdatePlaybackStatus(is_playing) => {
                self.is_playing = is_playing;
            }
            MediaPlayerMsg::PlayPause => {
                if let Some(ref player) = self.player {
                    let _ = player.checked_play_pause();
                }
            }
            MediaPlayerMsg::Next => {
                if let Some(ref player) = self.player {
                    let _ = player.checked_next();
                }
            }
            MediaPlayerMsg::Previous => {
                if let Some(ref player) = self.player {
                    let _ = player.checked_previous();
                }
            }
            MediaPlayerMsg::Refresh => {
                self.refresh_player_state();
                self.update_popover_content();
            }
            MediaPlayerMsg::TogglePopover => {
                self.popover.emit(PopoverMsg::Toggle);
            }
        }
    }
}

impl MediaPlayer {
    fn refresh_player_state(&mut self) {
        // Try to find an active player
        let player_finder = match PlayerFinder::new() {
            Ok(finder) => finder,
            Err(_) => return,
        };

        // Get the first active player
        let player = match player_finder.find_active() {
            Ok(player) => player,
            Err(_) => {
                // No active player, clear state
                self.player = None;
                self.track_title = String::new();
                self.track_artist = String::new();
                self.track_album = String::new();
                self.is_playing = false;
                return;
            }
        };

        // Update metadata
        if let Ok(metadata) = player.get_metadata() {
            let title = metadata
                .title()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Unknown Track".to_string());

            let artist = metadata
                .artists()
                .and_then(|artists| artists.first().map(|s| s.to_string()))
                .unwrap_or_else(|| "Unknown Artist".to_string());

            let album = metadata
                .album_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Unknown Album".to_string());

            self.track_title = title;
            self.track_artist = artist;
            self.track_album = album;
        }

        // Update playback status
        if let Ok(status) = player.get_playback_status() {
            self.is_playing = matches!(status, PlaybackStatus::Playing);
        }

        self.player = Some(player);
    }

    fn update_popover_content(&self) {
        // Update popover title with track name
        self.popover.emit(PopoverMsg::UpdateTitle(self.track_title.clone()));

        // Build items list
        let items = vec![
            PopoverItem::DetailRow {
                label: "Artist".to_string(),
                value: self.track_artist.clone(),
                value_css: "media-detail".to_string(),
            },
            PopoverItem::DetailRow {
                label: "Album".to_string(),
                value: self.track_album.clone(),
                value_css: "media-detail".to_string(),
            },
            PopoverItem::Separator,
            PopoverItem::DetailRow {
                label: "Status".to_string(),
                value: if self.is_playing { "Playing" } else { "Paused" }.to_string(),
                value_css: "media-detail".to_string(),
            },
        ];

        self.popover.emit(PopoverMsg::UpdateItems(items));
    }
}
