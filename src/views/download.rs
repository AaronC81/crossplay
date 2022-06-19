use std::{sync::{Arc, RwLock}, future::ready};

use iced::{Column, Text, Element, Command, Button, button, TextInput, text_input, Row, Container, container, Background, Length, alignment::Vertical, Rule};
use crate::{youtube::{YouTubeDownload, DownloadError}, Message, library::Library, ui_util::{ElementContainerExtensions, ContainerStyleSheet}};
use super::song_list::SongListMessage;

#[derive(Debug, Clone)]
pub enum DownloadMessage {
    IdInputChange(String),
    StartDownload,
    DownloadComplete(YouTubeDownload, Result<(), DownloadError>),
    ToggleStatus,
}

impl From<DownloadMessage> for Message {
    fn from(dm: DownloadMessage) -> Self { Message::DownloadMessage(dm) }
}

pub struct DownloadView {
    library: Arc<RwLock<Library>>,

    id_state: text_input::State,
    id_input: String,

    status_showing: bool,
    status_button_state: button::State,

    download_button_state: button::State,
    downloads_in_progress: Vec<YouTubeDownload>,
    download_errors: Vec<(YouTubeDownload, DownloadError)>,
    any_download_occurred: bool,
}

impl DownloadView {
    pub fn new(library: Arc<RwLock<Library>>) -> Self {
        Self {
            library,
            id_state: text_input::State::new(),
            id_input: "".to_string(),
            status_showing: false,
            status_button_state: button::State::new(),
            download_button_state: button::State::new(),
            downloads_in_progress: vec![],
            download_errors: vec![],
            any_download_occurred: false,
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        Column::new()
            .push(
                Container::new(
                    Row::new()
                        .spacing(15)
                        .padding(10)
                        .height(Length::Units(60))
                        .push(
                            TextInput::new(
                                &mut self.id_state, 
                                "Paste a YouTube video ID...", 
                                &self.id_input, 
                                |s| DownloadMessage::IdInputChange(s).into(),
                            )
                            .padding(5)
                        )
                        .push(
                            Button::new(
                                &mut self.download_button_state,
                                Text::new("Download")
                                    .vertical_alignment(Vertical::Center)
                                    .height(Length::Fill)
                            )
                            .on_press(DownloadMessage::StartDownload.into())
                            .height(Length::Fill)
                        )
                        .push(
                            Button::new(
                                &mut self.status_button_state,
                                Row::new()
                                    .height(Length::Fill)
                                    .push(
                                        Text::new(
                                            if !self.downloads_in_progress.is_empty() {
                                                format!("{} download(s) in progress", self.downloads_in_progress.len())
                                            } else if self.any_download_occurred {
                                                "All downloads complete".to_string()
                                            } else {
                                                "No downloads in progress".to_string()
                                            }
                                        )
                                            .vertical_alignment(Vertical::Center)
                                            .height(Length::Fill)
                                    )
                                    .push_if(!self.download_errors.is_empty(), ||
                                        Text::new(format!("{} download(s) failed", self.download_errors.len()))
                                            .vertical_alignment(Vertical::Center)
                                            .height(Length::Fill)
                                            .color([1.0, 0.0, 0.0])
                                    )
                                    .spacing(10)
                            )
                            .on_press(DownloadMessage::ToggleStatus.into())
                            .height(Length::Fill)
                        )
                )
                .style(ContainerStyleSheet(container::Style {
                    background: Some(Background::Color([0.85, 0.85, 0.85].into())),
                    ..Default::default()
                }))
            )
            .push_if(self.status_showing, ||
                Container::new(
                    Column::new()
                        .push(
                            Text::new(format!("{} download(s) in progress", self.downloads_in_progress.len()))
                        )
                        .push(
                            Column::with_children(self.downloads_in_progress.iter().map(|dl| {
                                Text::new(format!("ID {}", dl.id)).into()
                            }).collect())
                        )
                        .push(Rule::horizontal(10))
                        .push(
                            Column::with_children(if self.download_errors.is_empty() {
                                vec![Text::new("No download errors have occurred.").into()]
                            } else {
                                self.download_errors.iter().map(|(dl, err)| {
                                    Text::new(format!("Download {} failed: {:?}", dl.id, err)).color([1.0, 0.0, 0.0]).into()
                                }).collect()
                            })
                        )
                )
                .width(Length::Fill)
                .padding(10)
                .style(ContainerStyleSheet(container::Style {
                    background: Some(Background::Color([0.9, 0.9, 0.9].into())),
                    ..Default::default()
                }))
            )
            .into()
    }

    pub fn update(&mut self, message: DownloadMessage) -> Command<Message> { 
        match message {
            DownloadMessage::IdInputChange(s) => self.id_input = s,

            DownloadMessage::StartDownload => {
                self.any_download_occurred = true;

                // Need two named copies for the two closures
                let async_dl = YouTubeDownload::new(self.id_input.clone());
                let result_dl = async_dl.clone();
                self.downloads_in_progress.push(result_dl.clone());
                
                let library_path = self.library.read().unwrap().path.clone();
                return Command::perform(
                    (async move || {
                        async_dl.download(&library_path).await
                    })(),
                    move |r| DownloadMessage::DownloadComplete(result_dl.clone(), r).into()
                )
            },

            DownloadMessage::DownloadComplete(dl, result) => {
                // Remove the download which just finished
                self.downloads_in_progress.retain(|this_dl| *this_dl != dl);

                if let Err(e) = result {
                    self.download_errors.push((dl, e));
                }

                return Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
            },

            DownloadMessage::ToggleStatus => self.status_showing = !self.status_showing,
        }

        Command::none()
    }
}
