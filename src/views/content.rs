use std::sync::{RwLock, Arc};

use iced::{pure::Element, Subscription, Command};

use crate::{library::{Song, Library}, Message, settings::Settings};

use super::{song_list::{SongListMessage, SongListView}, crop::{CropView, CropMessage}, edit_metadata::{EditMetadataView, EditMetadataMessage}};

#[derive(Debug, Clone)]
pub enum ContentMessage {
    OpenSongList,
    OpenCrop(Song),
    OpenEditMetadata(Song),

    SongListMessage(SongListMessage),
    CropMessage(CropMessage),
    EditMetadataMessage(EditMetadataMessage),
}

impl From<ContentMessage> for Message {
    fn from(cm: ContentMessage) -> Self { Message::ContentMessage(cm) }
}

enum ContentViewState {
    SongList(SongListView),
    Crop(CropView),
    EditMetadata(EditMetadataView),
}

pub struct ContentView {
    library: Arc<RwLock<Library>>,
    settings: Arc<RwLock<Settings>>,

    state: ContentViewState,
}

impl ContentView {
    pub fn new(library: Arc<RwLock<Library>>, settings: Arc<RwLock<Settings>>) -> Self {
        Self {
            library: library.clone(),
            settings: settings.clone(),
            state: ContentViewState::SongList(SongListView::new(library, settings)),
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self.state {
            ContentViewState::SongList(ref v) => v.view(),
            ContentViewState::Crop(ref v) => v.view(),
            ContentViewState::EditMetadata(ref v) => v.view(),
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        match self.state {
            ContentViewState::Crop(ref v) => v.subscription(),
            _ => Subscription::none(),
        }
    }

    pub fn update(&mut self, message: ContentMessage) -> Command<Message> {
        match message {
            ContentMessage::OpenSongList => {
                self.library.write().unwrap().load_songs().unwrap();
                self.state = ContentViewState::SongList(
                    SongListView::new(self.library.clone(), self.settings.clone())
                );
            },

            ContentMessage::OpenCrop(song) =>
                self.state = ContentViewState::Crop(CropView::new(song)),
            ContentMessage::OpenEditMetadata(song) =>
                self.state = ContentViewState::EditMetadata(EditMetadataView::new(song)),

            ContentMessage::SongListMessage(m) =>
                if let ContentViewState::SongList(ref mut v) = self.state { return v.update(m); }
            ContentMessage::CropMessage(m) =>
                if let ContentViewState::Crop(ref mut v) = self.state { return v.update(m); }
            ContentMessage::EditMetadataMessage(m) =>
                if let ContentViewState::EditMetadata(ref mut v) = self.state { return v.update(m); }
        }

        Command::none()
    }
}
