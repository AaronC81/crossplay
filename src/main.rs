#![feature(async_closure)]

use std::{sync::Arc, marker::PhantomData, borrow::BorrowMut};

use iced::{Column, Text, Element, Settings, Application, executor, Command, Button, button, TextInput, text_input};
use library::Library;
use tokio::{sync::RwLock, task::JoinHandle};
use youtube::{YouTubeDownload, DownloadError};

mod youtube;
mod library;

fn main() {
    MainView::run(Settings::with_flags(())).unwrap();
}

#[derive(Debug, Clone)]
enum Message {
    ReloadSongList,
    DownloadIdInputChange(String),
    StartDownload,
    DownloadComplete(YouTubeDownload, Result<(), DownloadError>)
}

struct MainView {
    library: Arc<RwLock<Library>>,
    
    song_list_view: SongListView,

    download_id_state: text_input::State,
    download_id_input: String,
    download_button_state: button::State,
    downloads_in_progress: Vec<YouTubeDownload>,
    download_errors: Vec<(YouTubeDownload, DownloadError)>,
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
                download_button_state: button::State::new(),
                downloads_in_progress: vec![],
                download_errors: vec![],
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
            }
        }

        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        Column::new()
            .push(
                TextInput::new(
                    &mut self.download_id_state, 
                    "", 
                    &self.download_id_input, 
                    |s| Message::DownloadIdInputChange(s),
                )
            )
            .push(
                Button::new(
                    &mut self.download_button_state,
                    Text::new("Download")
                )
                .on_press(Message::StartDownload)
            )
            .push(
                Text::new(format!("{} download(s) in progress", self.downloads_in_progress.len()))
            )
            .push(Column::with_children(self.download_errors.iter().map(|(dl, err)| {
                Text::new(format!("Download {} failed: {:?}", dl.id, err)).color([1.0, 0.0, 0.0]).into()
            }).collect()))
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
