use std::{sync::{Arc, RwLock}, future::ready};

use iced::{Command, pure::{Element, widget::{Column, Text, Button, Rule}}};
use crate::{library::{Library, Song}, Message, ui_util::ElementContainerExtensions};

use super::content::ContentMessage;

#[derive(Debug, Clone)]
pub enum SongListMessage {
    RefreshSongList,
    RestoreOriginal(Song),
}

impl From<SongListMessage> for Message {
    fn from(slm: SongListMessage) -> Self { ContentMessage::SongListMessage(slm).into() }
}

pub struct SongListView {
    library: Arc<RwLock<Library>>,
    song_views: Vec<(Song, SongView)>,
}

impl SongListView {
    pub fn new(library: Arc<RwLock<Library>>) -> Self {        
        let mut song_views = vec![];
        Self::rebuild_song_views(library.clone(), &mut song_views);
        
        Self { library, song_views }
    }

    pub fn view(&self) -> Element<Message> {
        Column::new()
            .push(Column::with_children(
                self.song_views.iter().map(|x| Some(x)).intersperse_with(|| None).map(|view|
                    if let Some((_, view)) = view {
                        view.view().into()
                    } else {
                        Rule::horizontal(10).into()
                    }
                ).collect()
            ))
            .push(
                Button::new(Text::new("Reload song list"))
                    .on_press(SongListMessage::RefreshSongList.into())
            )
            .into()
    }

    pub fn update(&mut self, message: SongListMessage) -> Command<Message> {
        match message {
            SongListMessage::RefreshSongList => {
                // The content view does this for us!
                Command::perform(ready(()), |_| ContentMessage::OpenSongList.into())
            }

            SongListMessage::RestoreOriginal(song) => {
                song.restore_original_copy().unwrap();
                Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
            } 
        }
    }

    fn rebuild_song_views(library: Arc<RwLock<Library>>, views: &mut Vec<(Song, SongView)>) {
        views.clear();

        let library_reader = library.read().unwrap();
        let songs = library_reader.songs();

        for song in songs {
            views.push((song.clone(), SongView::new(library.clone(), song.clone())))
        }
    }
}

struct SongView {
    library: Arc<RwLock<Library>>,
    song: Song,
}

impl SongView {
    pub fn new(library: Arc<RwLock<Library>>, song: Song) -> Self {
        Self {
            library,
            song,
        }
    }

    pub fn view(&self) -> Element<Message> {
        Column::new()
            .push(Text::new(self.song.metadata.title.clone()))
            .push_if(self.song.metadata.is_cropped || self.song.metadata.is_metadata_edited, ||
                Button::new(Text::new("Restore original"))
                    .on_press(SongListMessage::RestoreOriginal(self.song.clone()).into()))
            .push(Button::new(Text::new("Edit metadata"))
                .on_press(ContentMessage::OpenEditMetadata(self.song.clone()).into()))
            .push_if(!self.song.metadata.is_cropped, ||
                Button::new(Text::new("Crop"))
                    .on_press(ContentMessage::OpenCrop(self.song.clone()).into()))
            .padding(10)
            .into()
    }
}
