use std::{sync::{Arc, RwLock}, time::Duration, future::ready};

use iced::{Column, Text, Element, Command, Button, button, Rule, Subscription, slider, Slider, Row};
use iced_futures::backend::default::time;
use iced_video_player::{VideoPlayer, VideoPlayerMessage};
use crate::{library::{Library, Song}, Message, ui_util::ElementContainerExtensions};
use url::Url;

#[derive(Debug, Clone)]
pub enum SongListMessage {
    RefreshSongList,

    EnterCropMode(Song),
    ExitCropMode,

    PlayPauseSong,
    SetSeekSongTarget(f64),
    SeekSong,
    TickPlayer,

    SetStart,
    JumpStart,
    SetEnd,
    JumpEnd,
    ApplyCrop,
    DeleteCrop(Song),

    VideoPlayerMessage(VideoPlayerMessage),
}

impl From<SongListMessage> for Message {
    fn from(slm: SongListMessage) -> Self { Message::SongListMessage(slm) }
}

enum SongListViewState {
    Normal {
        refresh_button: button::State,
        song_views: Vec<(Song, SongView)>,
    },
    CropMode {
        song: Song,
        player: VideoPlayer,

        song_progress_slider_state: slider::State,
        play_button_state: button::State,
        exit_button_state: button::State,
        seek_song_target: Option<(f64, bool)>,
        last_drawn_slider_position: f64,

        crop_start_point: Option<f64>,
        crop_end_point: Option<f64>,
        crop_set_start_button_state: button::State,
        crop_jump_start_button_state: button::State,
        crop_set_end_button_state: button::State,
        crop_jump_end_button_state: button::State,
        crop_apply_button_state: button::State,
    }
}

pub struct SongListView {
    library: Arc<RwLock<Library>>,
    state: SongListViewState,
}

impl SongListView {
    pub fn new(library: Arc<RwLock<Library>>) -> Self {
        let mut song_views = vec![];
        Self::rebuild_song_views(library.clone(), &mut song_views);
        
        Self {
            library,
            state: SongListViewState::Normal {
                refresh_button: button::State::new(),
                song_views,
            },
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        match &mut self.state {
            SongListViewState::Normal { ref mut refresh_button, song_views } =>
                Column::new()
                    .push(Column::with_children(
                        song_views.iter_mut().map(|x| Some(x)).intersperse_with(|| None).map(|view|
                            if let Some((song, view)) = view {
                                view.view().into()
                            } else {
                                Rule::horizontal(10).into()
                            }
                        ).collect()
                    ))
                    .push(
                        Button::new(refresh_button, Text::new("Reload song list"))
                            .on_press(SongListMessage::RefreshSongList.into())
                    )
                    .into(),

            SongListViewState::CropMode {
                song: _,
                player,

                song_progress_slider_state,
                play_button_state,
                exit_button_state,
                last_drawn_slider_position,
                seek_song_target,

                crop_start_point,
                crop_end_point,
                crop_set_start_button_state,
                crop_jump_start_button_state,
                crop_set_end_button_state,
                crop_jump_end_button_state,
                crop_apply_button_state,
            } =>
                Column::new()
                    .padding(10)
                    .spacing(10)
                    .push(player.frame_view())
                    .push(
                        Slider::new(
                            song_progress_slider_state,
                            0.0..=player.duration().as_millis() as f64,
                            {
                                if let Some((target, _)) = seek_song_target {
                                    *target
                                } else {
                                    let new_position = player.position().as_millis() as f64;
                                    if new_position > 0.0 {
                                        *last_drawn_slider_position = new_position;
                                        new_position
                                    } else {
                                        *last_drawn_slider_position
                                    }
                                }
                            },
                            |v| SongListMessage::SetSeekSongTarget(v).into(),
                        )
                            .on_release(SongListMessage::SeekSong.into())
                    )
                    .push(Button::new(play_button_state, Text::new(if player.paused() { "Play" } else { "Pause" }))
                        .on_press(SongListMessage::PlayPauseSong.into()))
                    .push(
                        Row::new()
                            .padding(10)
                            .push(Text::new("Start point:"))
                            .push(Button::new(crop_set_start_button_state, Text::new("Set"))
                                .on_press(SongListMessage::SetStart.into()))
                            .push_if(crop_start_point.is_some(), ||
                                Button::new(crop_jump_start_button_state, Text::new("Jump"))
                                    .on_press(SongListMessage::JumpStart.into()))
                            .push_if(crop_start_point.is_some(), ||
                                Text::new(format!("{}", crop_start_point.unwrap() / 1000.0)))
                    )
                    .push(
                        Row::new()
                            .padding(10)
                            .push(Text::new("End point:"))
                            .push(Button::new(crop_set_end_button_state, Text::new("Set"))
                                .on_press(SongListMessage::SetEnd.into()))
                            .push_if(crop_end_point.is_some(), ||
                                Button::new(crop_jump_end_button_state, Text::new("Jump"))
                                    .on_press(SongListMessage::JumpEnd.into()))
                            .push_if(crop_end_point.is_some(), ||
                                Text::new(format!("{}", crop_end_point.unwrap() / 1000.0)))
                    )
                    .push_if(
                        crop_start_point.is_some() && crop_end_point.is_some(),
                        || Button::new(crop_apply_button_state, Text::new("Apply and save"))
                            .on_press(SongListMessage::ApplyCrop.into()))
                    .push(Button::new(exit_button_state, Text::new("Cancel"))
                        .on_press(SongListMessage::ExitCropMode.into()))
                    .into(),

        }

    }

    pub fn subscription(&self) -> Subscription<Message> {
        if let SongListViewState::CropMode { .. } = self.state {
            time::every(Duration::from_millis(20)).map(|_| SongListMessage::TickPlayer.into())
        } else {
            Subscription::none()
        }
    }

    pub fn update(&mut self, message: SongListMessage) -> Command<Message> {
        match message {
            SongListMessage::RefreshSongList => {
                self.library.write().unwrap().load_songs().unwrap();
                
                if let SongListViewState::Normal { ref mut song_views, .. } = self.state {
                    Self::rebuild_song_views(self.library.clone(), song_views);
                }
            }

            SongListMessage::EnterCropMode(song) => {
                let mut player = VideoPlayer::new(
                    &Url::from_file_path(song.path.clone()).unwrap(),
                    false,
                ).unwrap();
                player.set_volume(0.2);
                player.set_paused(true);

                self.state = SongListViewState::CropMode {
                    song,
                    player,

                    song_progress_slider_state: slider::State::new(),
                    play_button_state: button::State::new(),
                    exit_button_state: button::State::new(),
                    last_drawn_slider_position: 0.0,
                    seek_song_target: None,

                    crop_start_point: None,
                    crop_end_point: None,
                    crop_set_start_button_state: button::State::new(),
                    crop_jump_start_button_state: button::State::new(),
                    crop_set_end_button_state: button::State::new(),
                    crop_jump_end_button_state: button::State::new(),
                    crop_apply_button_state: button::State::new(),
                }
            },

            SongListMessage::ExitCropMode => {
                self.library.write().unwrap().load_songs().unwrap();

                let mut song_views = vec![];
                Self::rebuild_song_views(self.library.clone(), &mut song_views);
                
                self.state = SongListViewState::Normal {
                    refresh_button: button::State::new(),
                    song_views,
                };
            }

            SongListMessage::PlayPauseSong => {
                if let SongListViewState::CropMode { player, .. } = &mut self.state {
                    player.set_paused(!player.paused());
                }
            },

            SongListMessage::SetSeekSongTarget(value) => {
                if let SongListViewState::CropMode { player, seek_song_target, .. } = &mut self.state {
                    *seek_song_target = Some(match seek_song_target {
                        // Was already seeking
                        Some((_, started_paused)) => (value, *started_paused),

                        // Just started seeking
                        None => (value, player.paused()),
                    });

                    player.set_paused(true);
                }
            }

            SongListMessage::SeekSong => {
                if let SongListViewState::CropMode { player, seek_song_target, .. } = &mut self.state {
                    if let Some((millis, already_paused)) = seek_song_target {
                        player.seek(Duration::from_secs_f64(*millis / 1000.0)).unwrap();
                        player.set_paused(*already_paused);
                    }
                    *seek_song_target = None;
                }
            }

            SongListMessage::TickPlayer => {
                // Don't need to do anything - the fact that a message has been sent is enough to 
                // update the UI
            }

            SongListMessage::SetStart => {
                if let SongListViewState::CropMode { player, crop_start_point, .. } = &mut self.state {
                    *crop_start_point = Some(player.position().as_millis() as f64);
                }
            }

            SongListMessage::JumpStart => {
                if let SongListViewState::CropMode { player, crop_start_point, .. } = &mut self.state {
                    if let Some(millis) = crop_start_point {
                        player.seek(Duration::from_secs_f64(*millis / 1000.0)).unwrap();
                    }
                }
            }

            SongListMessage::SetEnd => {
                if let SongListViewState::CropMode { player, crop_end_point, .. } = &mut self.state {
                    *crop_end_point = Some(player.position().as_millis() as f64);
                }
            }

            SongListMessage::JumpEnd => {
                if let SongListViewState::CropMode { player, crop_end_point, .. } = &mut self.state {
                    if let Some(millis) = crop_end_point {
                        player.seek(Duration::from_secs_f64(*millis / 1000.0)).unwrap();
                    }
                }
            }

            SongListMessage::ApplyCrop => {
                if let SongListViewState::CropMode { song, crop_start_point, crop_end_point, .. } = &mut self.state {
                    song.crop(Duration::from_secs_f64(crop_start_point.unwrap()), Duration::from_secs_f64(crop_end_point.unwrap())).unwrap();
                    return Command::perform(ready(()), |_| SongListMessage::ExitCropMode.into())
                }
            }

            SongListMessage::DeleteCrop(song) => {
                // TODO: will undo other modifications too, if/when we have those
                song.restore_original_copy().unwrap();
                return Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
            }

            SongListMessage::VideoPlayerMessage(msg) => {
                if let SongListViewState::CropMode { player, .. } = &mut self.state {
                    return player.update(msg).map(|m| SongListMessage::VideoPlayerMessage(m).into());
                }
            }
        }

        Command::none()
    }

    fn rebuild_song_views(library: Arc<RwLock<Library>>, views: &mut Vec<(Song, SongView)>) {
        views.clear();

        let library_reader = library.read().unwrap();
        let songs = library_reader.songs();

        for song in songs {
            views.push((song.clone(), SongView::new(library.clone(), song.clone())))
        }
    }
}

struct SongView {
    library: Arc<RwLock<Library>>,
    song: Song,
    delete_crop_button_state: button::State,
    crop_button_state: button::State,
}

impl SongView {
    pub fn new(library: Arc<RwLock<Library>>, song: Song) -> Self {
        Self {
            library,
            song,
            crop_button_state: button::State::new(),
            delete_crop_button_state: button::State::new(),
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        Column::new()
            .push(Text::new(self.song.metadata.title.clone()))
            .push_if(self.song.metadata.is_cropped, ||
                Button::new(&mut self.delete_crop_button_state, Text::new("Delete crop"))
                    .on_press(SongListMessage::DeleteCrop(self.song.clone()).into()))
            .push_if(!self.song.metadata.is_cropped, ||
                Button::new(&mut self.crop_button_state, Text::new("Crop"))
                    .on_press(SongListMessage::EnterCropMode(self.song.clone()).into()))
            .padding(10)
            .into()
    }
}
