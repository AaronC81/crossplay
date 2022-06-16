use std::sync::{Arc, RwLock};

use gtk::{traits::{OrientableExt, LabelExt}, Orientation::Vertical};
use relm::{Widget, Component, ContainerWidget, Update};
use relm_derive::{widget, Msg};

use crate::library::{Library, Song, self};

#[derive(Msg)]
pub enum SongListMsg {
    Refresh,
}

pub struct SongListModel {
    library: Arc<RwLock<Library>>,
    entries: Vec<Component<SongEntry>>,
}

#[widget]
impl Widget for SongList {
    fn model(library: Arc<RwLock<Library>>) -> SongListModel {
        SongListModel {
            library,
            entries: vec![],
        }
    }

    fn init_view(&mut self) {
        self.update(SongListMsg::Refresh);
    }

    fn update(&mut self, event: SongListMsg) {
        match event {
            SongListMsg::Refresh => {
                // Clear current song list
                for entry in self.model.entries.drain(..) {
                    self.widgets.song_list.remove_widget(entry);
                }

                // Build new song list
                let library = self.model.library.read().unwrap();
                for song in library.songs().cloned() {
                    let entry = self.widgets.song_list.add_widget::<SongEntry>(song);
                    self.model.entries.push(entry);
                }
            }
        }
    }

    view! {
        gtk::ScrolledWindow {
            #[name = "song_list"]
            gtk::Box {
                orientation: Vertical,
            },
        },
    }
}                

pub struct SongEntryModel {
    song: Song,
}

#[widget]
impl Widget for SongEntry {
    fn model(song: Song) -> SongEntryModel {
        SongEntryModel { song }
    }

    fn update(&mut self, _event: ()) {}

    view! {
        gtk::Label {
            text: &self.model.song.metadata.title,
        },
    }
}
