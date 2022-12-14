use std::{time::Duration, future::ready, cell::RefCell, cmp::max};

use iced::{Command, Subscription, time, pure::{Element, widget::{Column, Slider, Button, Text, Row, Container}}, Alignment, Length, Rule, Space, container::Style, Background};
use iced_video_player::{VideoPlayer, VideoPlayerMessage};
use url::Url;

use crate::{library::Song, Message, ui_util::{ButtonExtensions, ContainerStyleSheet}};

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

    seek_song_target: Option<(f64, bool)>,
    last_drawn_slider_position: RefCell<f64>,

    crop_start_point: Option<f64>,
    crop_end_point: Option<f64>,
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

            last_drawn_slider_position: RefCell::new(0.0),
            seek_song_target: None,

            crop_start_point: None,
            crop_end_point: None,
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

    pub fn view(&self) -> Element<Message> {
        Column::new()
            .padding(10)
            .spacing(10)
            .push(Text::new(format!("Crop: {}", self.song.metadata.title)).size(28))
            .push(self.player.frame_view()) // Actually invisible
            .push(self.player_controls())
            .push(Rule::horizontal(1))
            .push(
                Row::new()
                    .push(self.marker_display("Start", &self.crop_start_point, CropMessage::SetStart, CropMessage::JumpStart))
                    .push(self.marker_display("End", &self.crop_end_point, CropMessage::SetEnd, CropMessage::JumpEnd))
                    .height(Length::Shrink)
            )
            .push(
                Column::new()
                    .align_items(Alignment::Center)
                    .width(Length::Fill)
                    .push(
                        Row::new()
                            .padding(10)
                            .spacing(10)
                            .push(Button::new(Text::new("Cancel"))
                                .on_press(ContentMessage::OpenSongList.into()))
                            .push(Button::new(Text::new("Apply and save"))
                                .on_press_if(self.crop_start_point.is_some() && self.crop_end_point.is_some(), CropMessage::ApplyCrop.into()))
                    )
            )
            .into()
    }

    fn player_controls(&self) -> Element<Message> {
        Column::new()
            .align_items(Alignment::Center)
            .padding(10)
            .spacing(10)
            .push(Text::new("Media player").size(25))
            .push(
                Slider::new(
                    0.0..=self.player.duration().as_millis() as f64,
                    self.slider_millis(),
                    |v| CropMessage::SetSeekSongTarget(v).into(),
                )
                    .on_release(CropMessage::SeekSong.into())
            )
            .push(self.player_controls_markers())
            .push(Text::new(Self::render_millis(self.slider_millis())))
            .push(Button::new(Text::new(if self.player.paused() { "Play" } else { "Pause" }))
                .on_press(CropMessage::PlayPauseSong.into()))
            .into()
    }

    fn player_controls_markers(&self) -> Element<Message> {
        // This is, genuinely, one of the worst things I've ever written
        // I couldn't get SVG rendering at the width of the window to work consistently, so instead
        // we exploit a 1000-ish element flex-style layout to draw a line at any point along the
        // width of the screen

        fn pad(ratio: f64) -> Space {
            Space::with_width(Length::FillPortion(max((1000.0 * ratio).round() as u16, 1)))
        }

        fn pin<'a>(colour: [f32; 3]) -> Container<'a, Message> {
            Container::new(Space::new(Length::Units(1), Length::Units(20))).style(ContainerStyleSheet(Style {
                background: Some(Background::Color(colour.into())),
                ..Default::default()
            }))
        }

        match (self.crop_start_point, self.crop_end_point) {
            (None, None) => Row::new(),
            
            (Some(start), None) => Row::new()
                .push(pad(self.millis_ratio(start)))
                .push(pin([0.0, 0.0, 1.0]))
                .push(pad(1.0 - self.millis_ratio(start))),

            (None, Some(end)) => Row::new()
                .push(pad(self.millis_ratio(end)))
                .push(pin([1.0, 0.0, 0.0]))
                .push(pad(1.0 - self.millis_ratio(end))),

            (Some(start), Some(end)) => Row::new()
                .push(pad(self.millis_ratio(start)))
                .push(pin([0.0, 0.0, 1.0]))
                .push(pad(self.millis_ratio(end) - self.millis_ratio(start)))
                .push(pin([1.0, 0.0, 0.0]))
                .push(pad(1.0 - self.millis_ratio(end))),
        }
            .height(Length::Units(20))
            .into()
    }

    fn marker_display(&self, name: &str, value: &Option<f64>, set: CropMessage, jump: CropMessage) -> Element<Message> {
        Column::new()
            .align_items(Alignment::Center)
            .padding(10)
            .spacing(10)
            .width(Length::Fill)
            .push(Text::new(format!("{} point", name)).size(25))
            .push(
                Text::new(
                    if let Some(value) = value {
                        Self::render_millis(*value)
                    } else {
                        "Not set".to_string()
                    }
                )
            )
            .push(Button::new(Text::new("Set"))
                .on_press(set.into()))
            .push(Button::new(Text::new("Jump"))
                .on_press_if(value.is_some(), jump.into()))
            .into()
    }

    pub fn slider_millis(&self) -> f64 {
        if let Some((target, _)) = self.seek_song_target {
            target
        } else {
            let new_position = self.player.position().as_millis() as f64;
            if new_position > 0.0 {
                *self.last_drawn_slider_position.borrow_mut() = new_position;
                new_position
            } else {
                *self.last_drawn_slider_position.borrow()
            }
        }
    }

    pub fn millis_ratio(&self, millis: f64) -> f64 {
        millis / (self.player.duration().as_secs_f64() * 1000.0)
    }

    pub fn render_millis(millis: f64) -> String {
        let total_seconds = (millis / 1000.0).floor() as i32;

        let partitioned_minutes = total_seconds / 60;
        let partitioned_seconds = total_seconds % 60;
        let partitioned_millis = (millis % 1000.0).floor() as i32;

        format!("{:0>2}:{:0>2}:{:0>3}", partitioned_minutes, partitioned_seconds, partitioned_millis)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(20)).map(|_| CropMessage::TickPlayer.into())
    }
}
