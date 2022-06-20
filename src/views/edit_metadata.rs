use std::future::ready;

use iced::{Command, pure::{widget::{TextInput, Button, Column, Text, Row}, Element}, Length, Alignment, Image, image::Handle};

use crate::{library::Song, Message, ui_util::ElementContainerExtensions};

use super::content::ContentMessage;

#[derive(Debug, Clone)]
pub enum EditMetadataMessage {
    TitleChange(String),
    ArtistChange(String),
    AlbumChange(String),
    ApplyMetadataEdit,
}

impl From<EditMetadataMessage> for Message {
    fn from(emm: EditMetadataMessage) -> Self { Message::ContentMessage(ContentMessage::EditMetadataMessage(emm)) }
}

pub struct EditMetadataView {
    song: Song,
}

impl EditMetadataView {
    pub fn new(song: Song) -> Self {
        Self { song }
    }

    pub fn update(&mut self, message: EditMetadataMessage) -> Command<Message> {
        match message {
            EditMetadataMessage::TitleChange(v) => self.song.metadata.title = v,
            EditMetadataMessage::ArtistChange(v) => self.song.metadata.artist = v,
            EditMetadataMessage::AlbumChange(v) => self.song.metadata.album = v,

            EditMetadataMessage::ApplyMetadataEdit => {
                self.song.user_edit_metadata().unwrap();
                return Command::perform(ready(()), |_| ContentMessage::OpenSongList.into())
            }
        }

        Command::none()
    }

    pub fn view(&self) -> Element<Message> {
        Column::new()
            .padding(10)
            .spacing(10)
            .push(Text::new("Edit Metadata").size(28))
            .push(
                Row::new()
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .push_if_let(&self.song.metadata.album_art, |art|
                        Image::new(Handle::from_memory(art.data.clone()))
                            .width(Length::FillPortion(1))
                    )
                    .push(
                        Column::new()
                            .spacing(10)
                            .push(self.field("Title", &self.song.metadata.title, |v| EditMetadataMessage::TitleChange(v).into()))
                            .push(self.field("Artist", &self.song.metadata.artist, |v| EditMetadataMessage::ArtistChange(v).into()))
                            .push(self.field("Album", &self.song.metadata.album, |v| EditMetadataMessage::AlbumChange(v).into()))
                            .push(
                                Row::new()
                                    .spacing(10)
                                    .push(Button::new(Text::new("Cancel"))
                                        .on_press(ContentMessage::OpenSongList.into()))
                                    .push(Button::new(Text::new("Apply and save"))
                                        .on_press(EditMetadataMessage::ApplyMetadataEdit.into()))
                            )
                            .width(Length::FillPortion(2))
                    )
            )
            .into()
    }

    pub fn field<'a>(&'a self, label: &str, value: &str, func: impl Fn(String) -> Message + 'a) -> Element<Message> {
        Row::new()
            .spacing(10)
            .align_items(Alignment::Center)
            .push(Text::new(format!("{}:", label)).width(Length::Units(50)))
            .push(TextInput::new("", value, func).padding(5))
            .into()
    }
}
