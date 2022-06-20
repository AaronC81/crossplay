#![feature(async_closure)]
#![feature(iter_intersperse)]

use std::{sync::{Arc, RwLock}};

use iced::{Column, Element, Settings, Application, executor, Command, Subscription};
use library::Library;
use views::{download::{DownloadMessage, DownloadView}, song_list::{SongListMessage, SongListView}, content::{ContentMessage, ContentView}};

mod youtube;
mod library;
mod views;
mod ui_util;

fn main() {
    MainView::run(Settings::with_flags(())).unwrap();
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    DownloadMessage(DownloadMessage),
    ContentMessage(ContentMessage),
}

struct MainView {
    library: Arc<RwLock<Library>>,
    
    download_view: DownloadView,
    content_view: ContentView,
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

                download_view: DownloadView::new(library.clone()),
                content_view: ContentView::new(library.clone()),
            },
            Command::none()
        )
    }

    fn title(&self) -> String {
        "CrossPlay".to_string()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        self.content_view.subscription()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {         
        match message {
            Message::None => (),
            Message::ContentMessage(cm) => return self.content_view.update(cm),
            Message::DownloadMessage(dm) => return self.download_view.update(dm),
        }

        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        Column::new()
            .push(self.download_view.view())
            .push(self.content_view.view())
            .into()
    }
}
