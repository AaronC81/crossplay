#![feature(async_closure)]

use std::{sync::Arc, marker::PhantomData, borrow::BorrowMut};

use iced::{Column, Text, Element, Settings, Application, executor, Command, Button, button, TextInput, text_input, Row, Container, container, Background, Length, alignment::Vertical, Rule};
use library::Library;
use tokio::{sync::RwLock, task::JoinHandle};
use ui_util::ElementContainerExtensions;
use youtube::{YouTubeDownload, DownloadError};

mod youtube;
mod library;
mod ui_util;

fn main() {
    MainView::run(Settings::with_flags(())).unwrap();
}

#[derive(Debug, Clone)]
enum Message {
    ReloadSongList,
    DownloadIdInputChange(String),
    StartDownload,
    DownloadComplete(YouTubeDownload, Result<(), DownloadError>),
    ToggleDownloadStatus,
}

struct MainView {
    library: Arc<RwLock<Library>>,
    
    song_list_view: SongListView,

    download_id_state: text_input::State,
    download_id_input: String,
    download_status_showing: bool,
    download_status_button_state: button::State,
    download_button_state: button::State,
    downloads_in_progress: Vec<YouTubeDownload>,
    download_errors: Vec<(YouTubeDownload, DownloadError)>,
    any_download_occurred: bool,
}

impl Application for MainView {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let mut library = Library::new("/Users/aaron/Music/CrossPlay".into());
        library.load_songs().unwrap();
        let library = Arc::new(RwLock::new(library));
    
        (
            MainView {
                library: library.clone(),

                song_list_view: SongListView::new(library.clone()),

                download_id_state: text_input::State::new(),
                download_id_input: "".to_string(),
                download_status_showing: false,
                download_status_button_state: button::State::new(),
                download_button_state: button::State::new(),
                downloads_in_progress: vec![],
                download_errors: vec![],
                any_download_occurred: false,
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        "CrossPlay".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> { 
        match message {
            Message::ReloadSongList => self.library.blocking_write().load_songs().unwrap(),
            Message::DownloadIdInputChange(s) => self.download_id_input = s,

            Message::StartDownload => {
                self.any_download_occurred = true;

                // Need two named copies for the two closures
                let async_dl = YouTubeDownload::new(self.download_id_input.clone());
                let result_dl = async_dl.clone();
                self.downloads_in_progress.push(result_dl.clone());
                
                let library = self.library.clone();
                return Command::perform(
                    (async move || {
                        let library = library.read().await;
                        async_dl.download(library).await
                    })(),
                    move |r| Message::DownloadComplete(result_dl.clone(), r)
                )
            },

            Message::DownloadComplete(dl, result) => {
                // Remove the download which just finished
                self.downloads_in_progress.retain(|this_dl| *this_dl != dl);

                if let Err(e) = result {
                    self.download_errors.push((dl, e));
                }

                self.update(Message::ReloadSongList);
            },

            Message::ToggleDownloadStatus => self.download_status_showing = !self.download_status_showing,
        }

        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        Column::new()
            .push(
                Container::new(
                    Row::new()
                        .spacing(15)
                        .padding(10)
                        .height(Length::Units(60))
                        .push(
                            TextInput::new(
                                &mut self.download_id_state, 
                                "Paste a YouTube video ID...", 
                                &self.download_id_input, 
                                |s| Message::DownloadIdInputChange(s),
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
                            .on_press(Message::StartDownload)
                            .height(Length::Fill)
                        )
                        .push(
                            Button::new(
                                &mut self.download_status_button_state,
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
                            .on_press(Message::ToggleDownloadStatus)
                            .height(Length::Fill)
                        )
                )
                .style(ContainerStyleSheet(container::Style {
                    background: Some(Background::Color([0.85, 0.85, 0.85].into())),
                    ..Default::default()
                }))
            )
            .push_if(self.download_status_showing, ||
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
            .push(self.song_list_view.view())
            .into()
    }
}

struct SongListView {
    library: Arc<RwLock<Library>>,
    refresh_button: button::State,
}

impl SongListView {
    pub fn new(library: Arc<RwLock<Library>>) -> Self {
        Self {
            library,
            refresh_button: button::State::new(),
        }
    }

    pub fn view(&mut self) -> Element<Message> {
        let mut column = Column::new();

        for song in self.library.blocking_read().songs() {
            column = column.push(Text::new(song.metadata.title.clone()));
        }

        column = column.push(
            Button::new(&mut self.refresh_button, Text::new("Reload song list"))
                .on_press(Message::ReloadSongList)
        );

        column.into()
    }
}

struct ContainerStyleSheet(pub container::Style);
impl container::StyleSheet for ContainerStyleSheet { fn style(&self) -> container::Style { self.0 } }
