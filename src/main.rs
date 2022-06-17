#![feature(async_closure)]

use std::{sync::Arc, marker::PhantomData, borrow::BorrowMut};
use gtk::{Orientation::{Horizontal, Vertical}, traits::{OrientableExt, ButtonExt, LabelExt, WidgetExt, EntryExt}, Inhibit, EditableSignals};
use relm::{Widget, ContainerWidget, Relm, connect_async};
use relm_derive::{Msg, widget};

use library::Library;
use tokio::{sync::RwLock, task::JoinHandle};
use youtube::{YouTubeDownload, DownloadError};
use widgets::SongList;

mod youtube;
mod library;
mod widgets;

#[tokio::main]
async fn main() {
    let mut library = Library::new("/Users/aaron/Music/CrossPlay".into());
    library.load_songs().unwrap();
    let library = Arc::new(RwLock::new(library));

    TopLevelWindow::run(library).unwrap();
}


#[derive(Msg)]
pub enum TopLevelWindowMsg {
    InputDownloadId(String),
    StartDownload,
    DownloadComplete(YouTubeDownload),

    Quit,
}

pub struct TopLevelWindowModel {
    library: Arc<RwLock<Library>>,

    ongoing_downloads: Vec<(YouTubeDownload, JoinHandle<Result<(), DownloadError>>)>,
    download_id_input: String,
}

#[widget]
impl Widget for TopLevelWindow {
    fn model(relm: &Relm<Self>, library: Arc<RwLock<Library>>) -> TopLevelWindowModel {
        TopLevelWindowModel {
            library,

            ongoing_downloads: vec![],
            download_id_input: "".to_string(),
        }
    }

    fn update(&mut self, event: TopLevelWindowMsg) {
        match event {
            TopLevelWindowMsg::InputDownloadId(id) => self.model.download_id_input = id,
            TopLevelWindowMsg::StartDownload => {
                let dl = YouTubeDownload::new(self.model.download_id_input.clone());
                let library = self.model.library.clone();

                self.model.ongoing_downloads.push(
                    (
                        dl.clone(),
                        tokio::spawn(async move {
                            dl.download(library.read().await).await
                        }),
                    )
                );
            }
            TopLevelWindowMsg::DownloadComplete(d) => {
                println!("Download complete! {:?}", d)
            }

            TopLevelWindowMsg::Quit => gtk::main_quit(),
        }
    }

    view! {
        gtk::Window {
            gtk::Box {
                orientation: Vertical,

                // Download panel
                gtk::Box {
                    orientation: Horizontal,
                    gtk::Entry {
                        changed(entry) => TopLevelWindowMsg::InputDownloadId(entry.text().to_string()),
                        text: &self.model.download_id_input,
                    },
                    gtk::Button {
                        clicked => TopLevelWindowMsg::StartDownload,
                        label: "Download",
                    },
                },
                
                SongList(self.model.library.clone()),
            },

            // Use a tuple when you want to both send a message and return a value to
            // the GTK+ callback.
            delete_event(_, _) => (TopLevelWindowMsg::Quit, Inhibit(false)),
        }
    }
}

