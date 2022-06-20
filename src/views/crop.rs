use std::{time::Duration, future::ready};

use iced::{slider, button, Command, Element, Column, Slider, Button, Text, Row, Subscription, time};
use iced_video_player::{VideoPlayer, VideoPlayerMessage};
use url::Url;

use crate::{library::Song, Message, ui_util::ElementContainerExtensions};

use super::content::ContentMessage;

#[derive(Debug, Clone)]
pub enum CropMessage {
    PlayPauseSong,
    SetSeekSongTarget(f64),
    SeekSong,
    TickPlayer,

    SetStart,
    JumpStart,
    SetEnd,
    JumpEnd,
    ApplyCrop,

    VideoPlayerMessage(VideoPlayerMessage),
}

impl From<CropMessage> for Message {
    fn from(cm: CropMessage) -> Self { Message::ContentMessage(ContentMessage::CropMessage(cm)) }
}

pub struct CropView {
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

impl CropView {
    pub fn new(song: Song) -> Self {
        let mut player = VideoPlayer::new(
            &Url::from_file_path(song.path.clone()).unwrap(),
            false,
        ).unwrap();
        player.set_volume(0.2);
        player.set_paused(true);

        Self {
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
    }

    pub fn update(&mut self, message: CropMessage) -> Command<Message> {
        match message {
            CropMessage::PlayPauseSong => self.player.set_paused(!self.player.paused()),

            CropMessage::SetSeekSongTarget(value) => {
                self.seek_song_target = Some(match self.seek_song_target {
                    // Was already seeking
                    Some((_, started_paused)) => (value, started_paused),

                    // Just started seeking
                    None => (value, self.player.paused()),
                });

                self.player.set_paused(true);
            }

            CropMessage::SeekSong => {
                if let Some((millis, already_paused)) = self.seek_song_target {
                    self.player.seek(Duration::from_secs_f64(millis / 1000.0)).unwrap();
                    self.player.set_paused(already_paused);
                }
                self.seek_song_target = None;
            }

            CropMessage::TickPlayer => {
                // Don't need to do anything - the fact that a message has been sent is enough to 
                // update the UI
            }

            CropMessage::SetStart => 
                self.crop_start_point = Some(self.player.position().as_millis() as f64),
            CropMessage::JumpStart =>
                if let Some(millis) = self.crop_start_point {
                    self.player.seek(Duration::from_secs_f64(millis / 1000.0)).unwrap();
                },

            CropMessage::SetEnd =>
                self.crop_end_point = Some(self.player.position().as_millis() as f64),
            CropMessage::JumpEnd =>
                if let Some(millis) = self.crop_end_point {
                    self.player.seek(Duration::from_secs_f64(millis / 1000.0)).unwrap();
                },

            CropMessage::ApplyCrop => {
                self.song.crop(
                    Duration::from_secs_f64(self.crop_start_point.unwrap() / 1000.0),
                    Duration::from_secs_f64(self.crop_end_point.unwrap() / 1000.0)
                ).unwrap();
                return Command::perform(ready(()), |_| ContentMessage::OpenSongList.into())
            }

            CropMessage::VideoPlayerMessage(msg) => {
                return self.player.update(msg).map(|m| CropMessage::VideoPlayerMessage(m).into());
            }
        }

        Command::none()
    }

    pub fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(10)
            .spacing(10)
            .push(self.player.frame_view())
            .push(
                Slider::new(
                    &mut self.song_progress_slider_state,
                    0.0..=self.player.duration().as_millis() as f64,
                    {
                        if let Some((target, _)) = self.seek_song_target {
                            target
                        } else {
                            let new_position = self.player.position().as_millis() as f64;
                            if new_position > 0.0 {
                                self.last_drawn_slider_position = new_position;
                                new_position
                            } else {
                                self.last_drawn_slider_position
                            }
                        }
                    },
                    |v| CropMessage::SetSeekSongTarget(v).into(),
                )
                    .on_release(CropMessage::SeekSong.into())
            )
            .push(Button::new(&mut self.play_button_state, Text::new(if self.player.paused() { "Play" } else { "Pause" }))
                .on_press(CropMessage::PlayPauseSong.into()))
            .push(
                Row::new()
                    .padding(10)
                    .push(Text::new("Start point:"))
                    .push(Button::new(&mut self.crop_set_start_button_state, Text::new("Set"))
                        .on_press(CropMessage::SetStart.into()))
                    .push_if(self.crop_start_point.is_some(), ||
                        Button::new(&mut self.crop_jump_start_button_state, Text::new("Jump"))
                            .on_press(CropMessage::JumpStart.into()))
                    .push_if(self.crop_start_point.is_some(), ||
                        Text::new(format!("{}", self.crop_start_point.unwrap() / 1000.0)))
            )
            .push(
                Row::new()
                    .padding(10)
                    .push(Text::new("End point:"))
                    .push(Button::new(&mut self.crop_set_end_button_state, Text::new("Set"))
                        .on_press(CropMessage::SetEnd.into()))
                    .push_if(self.crop_end_point.is_some(), ||
                        Button::new(&mut self.crop_jump_end_button_state, Text::new("Jump"))
                            .on_press(CropMessage::JumpEnd.into()))
                    .push_if(self.crop_end_point.is_some(), ||
                        Text::new(format!("{}", self.crop_end_point.unwrap() / 1000.0)))
            )
            .push_if(
                self.crop_start_point.is_some() && self.crop_end_point.is_some(),
                || Button::new(&mut self.crop_apply_button_state, Text::new("Apply and save"))
                    .on_press(CropMessage::ApplyCrop.into()))
            .push(Button::new(&mut self.exit_button_state, Text::new("Cancel"))
                .on_press(ContentMessage::OpenSongList.into()))
            .into()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(20)).map(|_| CropMessage::TickPlayer.into())
    }
}
