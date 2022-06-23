use std::{sync::{Arc, RwLock}, future::ready, time::Duration};

use iced::{pure::{Element, widget::{Column, Text, Button, TextInput, Row, Container}}, container, Background, Length, alignment::Vertical, Rule, Command, ProgressBar, Subscription, time};
use crate::{youtube::{YouTubeDownload, DownloadError, YouTubeDownloadProgress, extract_video_id}, Message, library::Library, ui_util::{ElementContainerExtensions, ContainerStyleSheet}};
use super::song_list::SongListMessage;

#[derive(Debug, Clone)]
pub enum DownloadMessage {
    IdInputChange(String),
    StartDownload,
    DownloadComplete(YouTubeDownload, Result<(), DownloadError>),
    DismissErrors,
}

impl From<DownloadMessage> for Message {
    fn from(dm: DownloadMessage) -> Self { Message::DownloadMessage(dm) }
}

pub struct DownloadView {
    library: Arc<RwLock<Library>>,
    id_input: String,

    pub downloads_in_progress: Vec<(YouTubeDownload, Arc<RwLock<YouTubeDownloadProgress>>)>,
    download_errors: Vec<(YouTubeDownload, DownloadError)>,
}

impl DownloadView {
    pub fn new(library: Arc<RwLock<Library>>) -> Self {
        Self {
            library,
            id_input: "".to_string(),
            downloads_in_progress: vec![],
            download_errors: vec![],
        }
    }

    pub fn view(&self) -> Element<Message> {
        Column::new()
            .push(
                Container::new(
                    Row::new()
                        .spacing(15)
                        .padding(10)
                        .height(Length::Units(60))
                        .push(
                            TextInput::new(
                                "Paste a YouTube link...", 
                                &self.id_input, 
                                |s| DownloadMessage::IdInputChange(s).into(),
                            )
                            .padding(5)
                        )
                        .push(
                            Button::new(
                                Text::new("Download")
                                    .vertical_alignment(Vertical::Center)
                                    .height(Length::Fill)
                            )
                            .on_press(DownloadMessage::StartDownload.into())
                            .height(Length::Fill)
                        )
                )
                .style(ContainerStyleSheet(container::Style {
                    background: Some(Background::Color([0.85, 0.85, 0.85].into())),
                    ..Default::default()
                }))
            )
            .push_if(!self.downloads_in_progress.is_empty() || !self.download_errors.is_empty(), ||
                Container::new(
                    Column::new()
                        .push_if(!self.downloads_in_progress.is_empty(), ||
                            Text::new(format!("{} download(s) in progress...", self.downloads_in_progress.len()))
                        )
                        .push_if(!self.downloads_in_progress.is_empty(), ||
                            Column::with_children(self.downloads_in_progress.iter().map(|(dl, prog)| {
                                let prog = prog.read().unwrap();
                                let text = if let Some(metadata) = &prog.metadata {
                                    format!("{} (ID {})", metadata.title, dl.id)
                                } else {
                                    format!("Looking up video info... (ID {})", dl.id)
                                };

                                Row::new()
                                    .align_items(iced::Alignment::Center)
                                    .spacing(10)
                                    .width(Length::Fill)
                                    .push(
                                        ProgressBar::new(0.0..=100.0, prog.progress)
                                            .width(Length::FillPortion(2))
                                    )
                                    .push(Text::new(text).width(Length::FillPortion(3)))
                                    .into()
                            }).collect())
                                .spacing(10)
                        )
                        .push_if(!self.download_errors.is_empty(), ||
                            Column::new()
                                .push_if(!self.downloads_in_progress.is_empty(), || Rule::horizontal(10))
                                .push(
                                    Column::with_children(
                                        self.download_errors.iter().map(|(dl, err)| {
                                            Text::new(format!("Download {} failed: {:?}", dl.id, err)).color([1.0, 0.0, 0.0]).into()
                                        }).collect()
                                    )
                                )
                                .push(
                                    Button::new(Text::new("OK"))
                                        .on_press(DownloadMessage::DismissErrors.into())
                                )
                        )
                )
                .padding(10)
                .width(Length::Fill)
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
                // Need two named copies for the two closures
                let id = extract_video_id(&self.id_input);
                let async_dl = YouTubeDownload::new(id);
                let result_dl = async_dl.clone();
                let progress = Arc::new(RwLock::new(YouTubeDownloadProgress::new()));
                self.downloads_in_progress.push((result_dl.clone(), progress.clone()));

                self.id_input = "".to_string();
                
                let library_path = self.library.read().unwrap().path.clone();
                return Command::perform(
                    (async move || {
                        async_dl.download(&library_path, progress).await
                    })(),
                    move |r| DownloadMessage::DownloadComplete(result_dl.clone(), r).into()
                )
            },

            DownloadMessage::DownloadComplete(dl, result) => {
                // Remove the download which just finished
                self.downloads_in_progress.retain(|(this_dl, _)| *this_dl != dl);

                if let Err(e) = result {
                    self.download_errors.push((dl, e));
                }

                return Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
            },

            DownloadMessage::DismissErrors => self.download_errors.clear(),
        }

        Command::none()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // If a download is in progress, poke the UI to refresh occasionally to keep metadata and
        // progress up-to-date
        if !self.downloads_in_progress.is_empty() {
            time::every(Duration::from_millis(500)).map(|_| Message::None)
        } else {
            Subscription::none()
        }
    }
}
