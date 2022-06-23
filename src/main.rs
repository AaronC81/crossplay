#![feature(async_closure)]
#![feature(iter_intersperse)]

use std::{sync::{Arc, RwLock}};

use iced::{pure::{Element, widget::Column, Application}, executor, Command, Subscription};
use iced_native::{subscription, window, Event};
use library::Library;
use native_dialog::{MessageDialog, MessageType};
use settings::Settings;
use views::{download::{DownloadMessage, DownloadView}, content::{ContentMessage, ContentView}};

mod youtube;
mod library;
mod views;
mod ui_util;
mod settings;

fn main() {
    let mut settings = iced::Settings::with_flags(());
    settings.exit_on_close_request = false;

    MainView::run(settings).unwrap();
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    Close,
    DownloadMessage(DownloadMessage),
    ContentMessage(ContentMessage),
}

struct MainView {
    library: Arc<RwLock<Library>>,
    settings: Arc<RwLock<Settings>>,
    
    download_view: DownloadView,
    content_view: ContentView,
}

impl Application for MainView {
    type Message = Message;
    type Executor = executor::Default;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let settings = Settings::load();

        let mut library = Library::new(settings.library_path.clone());
        library.load_songs().unwrap();

        let library = Arc::new(RwLock::new(library));
        let settings = Arc::new(RwLock::new(settings));
    
        (
            MainView {
                library: library.clone(),
                settings,

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
        Subscription::batch([
            self.content_view.subscription(),
            self.download_view.subscription(),
            subscription::events().map(|e| {
                if let Event::Window(window::Event::CloseRequested) = e {
                    Message::Close
                } else {
                    Message::None
                }
            }),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {         
        match message {
            Message::None => (),
            Message::Close => {
                if self.download_view.downloads_in_progress.is_empty() {
                    std::process::exit(0);
                } else {
                    let confirmation = MessageDialog::new()
                        .set_title("Cancel downloads?")
                        .set_text(
                            "There are currently downloads in progress. Exiting now will cancel them. Are you sure you would like to exit?",
                        )
                        .set_type(MessageType::Warning)
                        .show_confirm()
                        .unwrap();

                    if confirmation {
                        std::process::exit(0);
                    }
                }
            },
            Message::ContentMessage(cm) => return self.content_view.update(cm),
            Message::DownloadMessage(dm) => return self.download_view.update(dm),
        }

        Command::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        Column::new()
            .push(self.download_view.view())
            .push(self.content_view.view())
            .into()
    }
}
