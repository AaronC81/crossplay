#![feature(async_closure)]
#![feature(iter_intersperse)]
#![feature(exit_status_error)]

use std::{sync::{Arc, RwLock}, future::ready};

use iced::{pure::{Element, widget::Column, Application}, executor, Command, Subscription};
use iced_native::{subscription, window, Event};
use library::Library;
use native_dialog::{MessageDialog, MessageType, FileDialog};
use settings::Settings;
use views::{download::{DownloadMessage, DownloadView}, content::{ContentMessage, ContentView}};

mod youtube;
mod library;
mod views;
mod ui_util;
mod settings;
mod assets;
mod tag_interface;

fn main() {
    let mut settings = iced::Settings::with_flags(());
    settings.exit_on_close_request = false;

    MainView::run(settings).unwrap();
}

#[derive(Debug, Clone)]
pub enum Message {
    None,
    Close,

    UpdateLibraryPath,

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
        let settings = Settings::load().unwrap();

        let mut library = Library::new(settings.library_path.clone());
        library.load_songs().unwrap();

        let library = Arc::new(RwLock::new(library));
        let settings = Arc::new(RwLock::new(settings));
    
        (
            MainView {
                library: library.clone(),
                settings: settings.clone(),

                download_view: DownloadView::new(library.clone(), settings.clone()),
                content_view: ContentView::new(library, settings),
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

            Message::UpdateLibraryPath => {
                let confirmation = MessageDialog::new()
                    .set_title("Pick new library?")
                    .set_text(&format!("Would you like to pick a new library folder? Your songs will not be copied to the new location, but will be preserved in the old location so you can switch back to it later.\n\nThe current library path is: {}", self.library.read().unwrap().path.to_string_lossy()))
                    .show_confirm();

                if !confirmation.unwrap() {
                    return Command::none();
                }

                if let Some(new_path) = FileDialog::new().show_open_single_dir().unwrap() {
                    let mut settings = self.settings.write().unwrap();
                    settings.library_path = new_path;
                    settings.save().unwrap();

                    self.library.write().unwrap().path = settings.library_path.clone();
                }

                return Command::perform(ready(()), |_| ContentMessage::OpenSongList.into())
            }
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
