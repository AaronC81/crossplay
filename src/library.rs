use std::{path::{PathBuf, Path}, fs::read_dir, rc::Rc, sync::Arc};

use id3::{Tag, TagLike, frame::Comment};

#[derive(Debug)]
pub struct Library {
    pub path: PathBuf,
    loaded_songs: Vec<Song>,
}

#[derive(Debug, Clone)]
pub enum LibraryError {
    IoError(Arc<std::io::Error>),
    TagError(Arc<id3::Error>),
}

impl Library {
    pub fn new(path: PathBuf) -> Self {
        Self { path, loaded_songs: vec![] }
    }
    
    pub fn songs(&self) -> impl Iterator<Item = &Song> {
        self.loaded_songs.iter()
    }

    pub fn load_songs(&mut self) -> Result<(), LibraryError> {
        // Look for MP3 files at the root of the directory
        self.loaded_songs.clear();
        let entries = read_dir(&self.path).map_err(|e| LibraryError::IoError(Arc::new(e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| LibraryError::IoError(Arc::new(e)))?;
            let path = entry.path();

            if path.extension().map(|s| s.to_ascii_lowercase()) == Some("mp3".into()) {
                let tag = Tag::read_from_path(&path);
        
                // If there's no video ID, then this didn't come from CrossPlay, so ignore it
                if let Ok(tag) = tag {
                    if let Some(video_id) = SongMetadata::get_youtube_id(&tag) {
                        let metadata = SongMetadata {
                            title: tag.title().unwrap_or("Unknown Title").into(),
                            artist: tag.artist().unwrap_or("Unknown Artist").into(),
                            album: tag.artist().unwrap_or("Unknown Album").into(),
                            youtube_id: video_id.text.into(),
                        };

                        self.loaded_songs.push(Song::new(path, metadata));
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Song {
    pub path: PathBuf,
    pub metadata: SongMetadata,
}

impl Song {
    fn new(path: PathBuf, metadata: SongMetadata) -> Self {
        Self { path, metadata }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub youtube_id: String,
}

const TAG_KEY_YOUTUBE_ID: &str = "[CrossPlay] YouTube ID";

impl SongMetadata {
    fn get_youtube_id(tag: &Tag) -> Option<Comment> {
        tag.comments().find(|c| { c.description == TAG_KEY_YOUTUBE_ID }).map(|c| c.clone())
    }
    
    fn set_youtube_id(&self, tag: &mut Tag) {
        // If there's already an ID, remove it
        if let Some(comment) = Self::get_youtube_id(tag) {
            tag.remove_comment(Some(&comment.description), Some(&comment.text))
        }

        tag.add_frame(Comment {
            lang: "eng".into(),
            description: TAG_KEY_YOUTUBE_ID.into(),
            text: self.youtube_id.clone(),
        });
    }

    fn write_into_tag(&self, tag: &mut Tag) {
        tag.set_title(self.title.clone());
        tag.set_artist(self.artist.clone());
        tag.set_album(self.album.clone());

        self.set_youtube_id(tag);
    }

    pub(crate) fn write_into_file(&self, file: &Path) -> Result<(), LibraryError> {
        let mut tag = Tag::new();
        self.write_into_tag(&mut tag);
        Tag::write_to_path(&tag, file, id3::Version::Id3v23).map_err(|e| LibraryError::TagError(Arc::new(e)))?;
        Ok(())
    }
}
