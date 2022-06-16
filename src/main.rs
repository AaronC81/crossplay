#![feature(async_closure)]

use std::{sync::{Arc, RwLock}, marker::PhantomData, borrow::BorrowMut};
use gtk::{Orientation::{Horizontal, Vertical}, traits::{OrientableExt, ButtonExt, LabelExt, WidgetExt, EntryExt}, Inhibit, EditableSignals};
use relm::{Widget, ContainerWidget};
use relm_derive::{Msg, widget};

use library::Library;
use youtube::YouTubeDownload;
use widgets::SongList;

mod youtube;
mod library;
mod widgets;

fn main() {
    let mut library = Library::new("/Users/aaron/Music/CrossPlay".into());
    library.load_songs().unwrap();
    let library = Arc::new(RwLock::new(library));

    TopLevelWindow::run(library).unwrap();
}


#[derive(Msg)]
pub enum TopLevelWindowMsg {
    InputDownloadId(String),
    StartDownload,

    Quit,
}

pub struct TopLevelWindowModel {
    library: Arc<RwLock<Library>>,

    download_id_input: String,
}

#[widget]
impl Widget for TopLevelWindow {
    fn model(library: Arc<RwLock<Library>>) -> TopLevelWindowModel {
        TopLevelWindowModel {
            library,
            download_id_input: "".to_string(),
        }
    }

    fn update(&mut self, event: TopLevelWindowMsg) {
        match event {
            TopLevelWindowMsg::InputDownloadId(id) => self.model.download_id_input = id,
            TopLevelWindowMsg::StartDownload => println!("Downloading {}", self.model.download_id_input.to_string()),

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

