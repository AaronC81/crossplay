#![feature(async_closure)]
#![feature(iter_intersperse)]

use std::{sync::{Arc, RwLock, Mutex}, future::ready, path::PathBuf, io::BufReader, fs::File};

use iced::{Column, Text, Element, Settings, Application, executor, Command, Button, button, TextInput, text_input, Row, Container, container, Background, Length, alignment::Vertical, Rule};
use library::{Library, Song};
use rodio::{OutputStream, Decoder, Source, Sink, OutputStreamHandle};
use ui_util::ElementContainerExtensions;
use youtube::{YouTubeDownload, DownloadError};

mod youtube;
mod library;
mod ui_util;

static mut OUTPUT_STREAM: Option<OutputStream> = None;
static mut OUTPUT_STREAM_HANDLE: Option<OutputStreamHandle> = None;

fn main() {
    unsafe {
        let (stream, stream_handle) = OutputStream::try_default().unwrap();
        OUTPUT_STREAM = Some(stream);
        OUTPUT_STREAM_HANDLE = Some(stream_handle);
    }
    MainView::run(Settings::with_flags(())).unwrap();
}

#[derive(Debug, Clone)]
enum Message {
    None,
    ReloadSongList,
    DownloadMessage(DownloadMessage),
    SongListMessage(SongListMessage),
}

struct MainView {
    library: Arc<RwLock<Library>>,
    
    song_list_view: SongListView,
    download_view: DownloadView,
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
                download_view: DownloadView::new(library.clone()),
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        "CrossPlay".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> { 
        match message {
            Message::None => (),
            Message::ReloadSongList => {
                self.library.write().unwrap().load_songs().unwrap();
                self.song_list_view.rebuild_song_views();
            },
            Message::SongListMessage(slm) => return self.song_list_view.update(slm),
            Message::DownloadMessage(dm) => return self.download_view.update(dm),
        }

        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        Column::new()
            .push(self.download_view.view())
            .push(self.song_list_view.view())
            .into()
    }
}

#[derive(Debug, Clone)]
enum DownloadMessage {
    IdInputChange(String),
    StartDownload,
    DownloadComplete(YouTubeDownload, Result<(), DownloadError>),
    ToggleStatus,
}

impl From<DownloadMessage> for Message {
    fn from(dm: DownloadMessage) -> Self { Message::DownloadMessage(dm) }
}

struct DownloadView {
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

                return Command::perform(ready(()), |_| Message::ReloadSongList)
            },

            DownloadMessage::ToggleStatus => self.status_showing = !self.status_showing,
        }

        Command::none()
    }
}

#[derive(Debug, Clone)]
enum SongListMessage {
    PlaySong(Song),
    PlaySongWorker,
    StopSong,
}

impl From<SongListMessage> for Message {
    fn from(slm: SongListMessage) -> Self { Message::SongListMessage(slm) }
}

struct SongListView {
    library: Arc<RwLock<Library>>,
    refresh_button: button::State,
    song_views: Vec<(Song, SongView)>,
    currently_playing_song: Arc<RwLock<Option<(Song, Sink)>>>,
}

impl SongListView {
    pub fn new(library: Arc<RwLock<Library>>) -> Self {
        let mut result = Self {
            library,
            refresh_button: button::State::new(),
            song_views: vec![],
            currently_playing_song: Arc::new(RwLock::new(None)),
        };
        result.rebuild_song_views();
        result
    }

    pub fn view(&mut self) -> Element<Message> {
        let currently_playing_song = self.currently_playing_song.read().unwrap();
        let currently_playing_song = currently_playing_song.as_ref().map(|x| &x.0);

        Column::new()
            .push(Column::with_children(
                self.song_views.iter_mut().map(|x| Some(x)).intersperse_with(|| None).map(|view|
                    if let Some((song, view)) = view {
                        view.view(Some(&*song) == currently_playing_song).into()
                    } else {
                        Rule::horizontal(10).into()
                    }
                ).collect()
            ))
            .push(
                Button::new(&mut self.refresh_button, Text::new("Reload song list"))
                    .on_press(Message::ReloadSongList)
            )
            .into()
    }

    pub fn update(&mut self, message: SongListMessage) -> Command<Message> {
        match message {
            SongListMessage::PlaySong(song) => {
                let currently_playing_song = self.currently_playing_song.clone();

                if currently_playing_song.read().unwrap().is_some() {
                    return Command::none();
                }

                return Command::perform((async move || {
                    // Safety: The `if` above would've bailed if something else is playing audio, so
                    // we're definitely the only thread doing so.
                    let stream_handle = unsafe { OUTPUT_STREAM_HANDLE.as_ref().unwrap() };

                    let sink = Sink::try_new(stream_handle).unwrap();
                    let file = BufReader::new(File::open(song.path.clone()).unwrap());
                    let source = Decoder::new(file).unwrap();
                    sink.set_volume(0.1);
                    sink.append(source);

                    *currently_playing_song.write().unwrap() = Some((song, sink));
                })(), |_| SongListMessage::PlaySongWorker.into());
            },

            SongListMessage::PlaySongWorker => {
                let currently_playing_song = self.currently_playing_song.clone();

                return Command::perform((async move || {
                    currently_playing_song.read().unwrap().as_ref().unwrap().1.sleep_until_end();
                })(), |_| Message::None);
            },

            SongListMessage::StopSong => {
                let currently_playing_song = self.currently_playing_song.read().unwrap();
                if let Some((_, sink)) = &*currently_playing_song {
                    sink.stop();
                }
                drop(currently_playing_song);

                let mut currently_playing_song = self.currently_playing_song.write().unwrap();
                *currently_playing_song = None;
            },
        }

        Command::none()
    }

    pub fn rebuild_song_views(&mut self) {
        self.song_views.clear();

        let library = self.library.read().unwrap();
        let songs = library.songs();

        for song in songs {
            self.song_views.push((song.clone(), SongView::new(self.library.clone(), song.clone())))
        }
    }
}

struct SongView {
    library: Arc<RwLock<Library>>,
    song: Song,
    play_button_state: button::State,
}

impl SongView {
    pub fn new(library: Arc<RwLock<Library>>, song: Song) -> Self {
        Self {
            library,
            song,
            play_button_state: button::State::new(),
        }
    }

    pub fn view(&mut self, playing: bool) -> Element<Message> {
        Column::new()
            .push(Text::new(self.song.metadata.title.clone()))
            .push(
                Button::new(&mut self.play_button_state, Text::new(if playing { "Stop" } else { "Play" }))
                    .on_press(
                        if playing {
                            SongListMessage::StopSong.into()
                        } else {
                            SongListMessage::PlaySong(self.song.clone()).into()
                        }
                    )
            )
            .padding(10)
            .into()
    }
}

struct ContainerStyleSheet(pub container::Style);
impl container::StyleSheet for ContainerStyleSheet { fn style(&self) -> container::Style { self.0 } }
