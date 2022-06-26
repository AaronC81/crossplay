use std::{sync::{Arc, RwLock}, future::ready};

use iced::{Command, pure::{Element, widget::{Column, Text, Button, Rule, Row, Image, Scrollable}}, image::Handle, Space, Length, Alignment};
use native_dialog::{MessageDialog, MessageType};
use crate::{library::{Library, Song}, Message, ui_util::{ElementContainerExtensions, ButtonExtensions}, settings::{Settings, SortBy, SortDirection}, assets};

use super::content::ContentMessage;

#[derive(Debug, Clone)]
pub enum SongListMessage {
    RefreshSongList,
    ChangeSort(SortBy),
    ToggleSortReverse,

    RestoreOriginal(Song),
    Delete(Song),
    ToggleHide(Song),
}

impl From<SongListMessage> for Message {
    fn from(slm: SongListMessage) -> Self { ContentMessage::SongListMessage(slm).into() }
}

pub struct SongListView {
    library: Arc<RwLock<Library>>,
    settings: Arc<RwLock<Settings>>,

    song_views: Vec<(Song, SongView)>,
}

impl SongListView {
    pub fn new(library: Arc<RwLock<Library>>, settings: Arc<RwLock<Settings>>) -> Self {
        let mut result = Self { library, settings, song_views: vec![] };
        result.rebuild_song_views();
        result
    }

    pub fn view(&self) -> Element<Message> {
        Scrollable::new(
            Column::new()
                .align_items(Alignment::Center)
                .spacing(10)
                .push(Column::with_children(
                    self.song_views
                        .iter()
                        .map(Some)
                        .intersperse_with(|| None)
                        .map(|view|
                            if let Some((_, view)) = view {
                                view.view()
                            } else {
                                Rule::horizontal(10).into()
                            }
                        )
                        .collect()
                ))
        ).into()
    }

    pub fn update(&mut self, message: SongListMessage) -> Command<Message> {
        match message {
            SongListMessage::RefreshSongList => {
                // The content view does this for us!
                Command::perform(ready(()), |_| ContentMessage::OpenSongList.into())
            }

            SongListMessage::ChangeSort(sort) => {
                let mut settings = self.settings.write().unwrap();
                settings.sort_by = sort;
                settings.save().expect("failed to save settings");
                drop(settings);

                self.sort_song_views();

                Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
            }

            SongListMessage::ToggleSortReverse => {
                let mut settings = self.settings.write().unwrap();
                settings.sort_direction = settings.sort_direction.reverse();
                settings.save().expect("failed to save settings");
                drop(settings);

                self.sort_song_views();

                Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
            }

            SongListMessage::RestoreOriginal(song) => {
                let confirmation = MessageDialog::new()
                    .set_title("Restore original?")
                    .set_text(&format!(
                        "This will undo any metadata modifications, and remove the crop if applied. Are you sure you would like to restore '{}'?",
                        song.metadata.title,
                    ))
                    .set_type(MessageType::Warning)
                    .show_confirm()
                    .unwrap();

                if confirmation {
                    song.restore_original_copy().unwrap();
                    Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
                } else {
                    Command::none()
                }
            }

            SongListMessage::Delete(mut song) => {
                let confirmation = MessageDialog::new()
                    .set_title("Delete song?")
                    .set_text(&format!(
                        "This will permanently delete the song and any modifications made to it. Are you sure you would like to delete '{}'?",
                        song.metadata.title,
                    ))
                    .set_type(MessageType::Warning)
                    .show_confirm()
                    .unwrap();

                if confirmation {
                    song.delete().expect("delete failed");
                    Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
                } else {
                    Command::none()
                }
            }

            SongListMessage::ToggleHide(song) => {
                let mut need_refresh = false;

                if song.is_hidden() {
                    let confirmation = MessageDialog::new()
                        .set_title("Unhide song?")
                        .set_text(&format!(
                            "The song '{}' will re-appear in media players.",
                            song.metadata.title,
                        ))
                        .set_type(MessageType::Warning)
                        .show_confirm()
                        .unwrap();

                    if confirmation {
                        song.unhide().expect("unhide failed");
                        need_refresh = true;
                    }
                } else {
                    let confirmation = MessageDialog::new()
                        .set_title("Hide song?")
                        .set_text(&format!(
                            "The song '{}' will remain downloaded and visible in CrossPlay, but will stop showing in media players.",
                            song.metadata.title,
                        ))
                        .set_type(MessageType::Warning)
                        .show_confirm()
                        .unwrap();

                    if confirmation {
                        song.hide().expect("hide failed");
                        need_refresh = true;
                    }
                }

                if need_refresh {
                    Command::perform(ready(()), |_| SongListMessage::RefreshSongList.into())
                } else {
                    Command::none()
                }
            }
        }
    }

    fn rebuild_song_views(&mut self) {
        self.song_views.clear();

        let library_reader = self.library.read().unwrap();
        let songs = library_reader.songs();

        for song in songs {
            self.song_views.push((song.clone(), SongView::new(self.library.clone(), song.clone())))
        }

        drop(library_reader);

        self.sort_song_views();
    }

    fn sort_song_views(&mut self) {
        let settings = self.settings.read().unwrap();
        
        match settings.sort_by {
            SortBy::Title => self.song_views.sort_by_key(|(s, _)| s.metadata.title.clone().to_lowercase()),
            SortBy::Artist => self.song_views.sort_by_key(|(s, _)| s.metadata.artist.clone().to_lowercase()),
            SortBy::Album => self.song_views.sort_by_key(|(s, _)| s.metadata.album.clone().to_lowercase()),
            
            // It makes sense for the default order of download time to go from newest to oldest,
            // so "invert" the u64 by subtracting it from the largest possible
            SortBy::Downloaded => self.song_views.sort_by_key(|(s, _)| u64::MAX - s.metadata.download_unix_time),
        }

        match settings.sort_direction {
            SortDirection::Normal => (),
            SortDirection::Reverse => self.song_views.reverse(),
        }
    }
}

#[allow(unused)]
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
        Row::new()
            .padding(10)
            .spacing(10)
            .align_items(Alignment::Center)
            .push_if_let(&self.song.metadata.album_art, |art|
                Image::new(Handle::from_memory(art.data.clone()))
                    .width(Length::Units(100))
            )
            .push(
                Column::new()
                    .push(Text::new(self.song.metadata.title.clone()))
                    .push(Text::new(self.song.metadata.artist.clone()).color([0.3, 0.3, 0.3]))
            )
            .push(Space::with_width(Length::Fill))
            // TODO: these buttons aren't responsive at all!
            // Too long a title will cause these to go tiny
            .push(
                Button::new(Image::new(assets::EDIT))
                    .on_press(ContentMessage::OpenEditMetadata(self.song.clone()).into())
                    .width(Length::Units(40))
            )
            .push(
                Button::new(Image::new(if self.song.metadata.is_cropped { assets::CROP_DISABLED } else { assets::CROP }))
                    .on_press_if(!self.song.metadata.is_cropped, ContentMessage::OpenCrop(self.song.clone()).into())
                    .width(Length::Units(40))
            )
            .push(
                Button::new(Image::new(if self.song.is_hidden() { assets::HIDDEN } else { assets::NOT_HIDDEN }))
                    .on_press(SongListMessage::ToggleHide(self.song.clone()).into())
                    .width(Length::Units(40))
            )
            .push(
                Button::new(Image::new(if self.song.is_modified() { assets::RESTORE } else { assets::RESTORE_DISABLED }))
                    .on_press_if(self.song.is_modified(), SongListMessage::RestoreOriginal(self.song.clone()).into())
                    .width(Length::Units(40))
            )
            .push(
                Button::new(Image::new(assets::DELETE))
                    .on_press(SongListMessage::Delete(self.song.clone()).into())
                    .width(Length::Units(40))
            )
            .into()
    }
}
