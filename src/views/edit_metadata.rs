use std::future::ready;

use iced::{text_input, button, Command, Element, Column, TextInput, Button, Text};

use crate::{library::Song, Message};

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

    title_text_input: text_input::State,
    artist_text_input: text_input::State,
    album_text_input: text_input::State,
    apply_button_state: button::State,
}

impl EditMetadataView {
    pub fn new(song: Song) -> Self {
        Self {
            song,
            title_text_input: text_input::State::new(),
            artist_text_input: text_input::State::new(),
            album_text_input: text_input::State::new(),
            apply_button_state: button::State::new(),
        }
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

    pub fn view(&mut self) -> Element<Message> {
        Column::new()
            .padding(10)
            .spacing(10)
            .push(TextInput::new(&mut self.title_text_input, "", &self.song.metadata.title, |v| EditMetadataMessage::TitleChange(v).into()))
            .push(TextInput::new(&mut self.artist_text_input, "", &self.song.metadata.artist, |v| EditMetadataMessage::ArtistChange(v).into()))
            .push(TextInput::new(&mut self.album_text_input, "", &self.song.metadata.album, |v| EditMetadataMessage::AlbumChange(v).into()))
            .push(Button::new(&mut self.apply_button_state, Text::new("Apply and save"))
                .on_press(EditMetadataMessage::ApplyMetadataEdit.into()))
            .into()
    }
}
