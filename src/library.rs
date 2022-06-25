use std::{path::{PathBuf, Path}, fs::read_dir, time::Duration, process::Command};

use anyhow::Result;
use id3::{Tag, TagLike, frame::{Picture, PictureType}};

use crate::tag_interface::{YouTubeIdTag, DownloadTimeTag, CroppedTag, MetadataEditedTag, CustomTagExtensions};

/// A collection of songs, managed by CrossPlay, saved to a particular location.
/// 
/// To avoid extraneous I/O calls, each library instance stores a [`Vec`] of loaded songs. Care must
/// be taken to reload this whenever necessary so that the application is not acting on a stale
/// state.
#[derive(Debug)]
pub struct Library {
    pub path: PathBuf,
    loaded_songs: Vec<Song>,
}

impl Library {
    /// Creates a new reference to a library on-disk.
    pub fn new(path: PathBuf) -> Self {
        Self { path, loaded_songs: vec![] }
    }
    
    /// Iterates over all loaded songs.
    /// 
    /// You must call [`load_songs`] before this.
    pub fn songs(&self) -> impl Iterator<Item = &Song> {
        self.loaded_songs.iter()
    }

    /// Reloads the list of songs in this library.
    /// 
    /// For a song to be loaded, it must:
    ///   - Be in the root of the library folder
    ///   - Be an MP3 file with a .mp3 extension
    ///   - Have a CrossPlay video ID comment in its ID3 tags
    pub fn load_songs(&mut self) -> Result<()> {
        // Look for MP3 files at the root of the directory
        self.loaded_songs.clear();
        let entries = read_dir(&self.path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|s| s.to_ascii_lowercase()) == Some("mp3".into()) {
                let tag = Tag::read_from_path(&path);
        
                // If there's no video ID, then this didn't come from CrossPlay, so ignore it
                if let Ok(tag) = tag {
                    if let Ok(metadata) = Self::load_one_song_metadata(tag) {
                        self.loaded_songs.push(Song::new(path, metadata));
                    }
                }
            }
        }

        Ok(())
    }

    fn load_one_song_metadata(tag: Tag) -> Result<SongMetadata> {            
        Ok(SongMetadata {
            title: tag.title().unwrap_or("Unknown Title").into(),
            artist: tag.artist().unwrap_or("Unknown Artist").into(),
            album: tag.album().unwrap_or("Unknown Album").into(),
            youtube_id: tag.read_custom::<YouTubeIdTag>()?,
            album_art: SongMetadata::get_album_art(&tag),
            is_cropped: tag.read_custom::<CroppedTag>()?,
            is_metadata_edited: tag.read_custom::<MetadataEditedTag>()?,
            download_unix_time: tag.read_custom::<DownloadTimeTag>()?,
        })
    }
}

/// A song loaded from a library.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Song {
    /// The path to the working copy of this song, possibly modified.
    pub path: PathBuf,

    /// This song's metadata, loaded from ID3 tags.
    pub metadata: SongMetadata,
}

impl Song {
    /// Creates a new reference to a song on-disk.
    fn new(path: PathBuf, metadata: SongMetadata) -> Self {
        Self { path, metadata }
    }

    /// The path where the original of this song will be copied to, before any modifications take
    /// place.
    /// 
    /// This will not exist if the song has not been modified (and thus [`create_original_copy`] has
    /// not been called).
    fn original_copy_path(&self) -> PathBuf {
        format!("{}.original", self.path.to_string_lossy()).into()
    }

    /// Creates an original copy of this song, if one does not already exist. It is the caller's
    /// responsibility to ensure this is called before modifying the file at the song's [`path`].
    fn create_original_copy(&self) -> Result<()> {
        if self.original_copy_path().exists() { return Ok(()) }
        std::fs::copy(&self.path, self.original_copy_path())?;

        Ok(())
    }

    /// Restores the original copy of this song, replacing the working copy. The original copy is
    /// left intact.
    /// 
    /// Errors if an original does not exist.
    pub fn restore_original_copy(&self) -> Result<()> {
        std::fs::copy(self.original_copy_path(), &self.path)?;
        Ok(())
    }

    /// Returns true if this song's metadata indicates that it has been modified from the original.
    pub fn is_modified(&self) -> bool {
        self.metadata.is_cropped || self.metadata.is_metadata_edited
    }

    /// Modifies the working copy of this song to start and end at the selected points. This is
    /// accomplished by shelling out to ffmpeg.
    /// 
    /// Also sets the [`SongMetadata.is_cropped`] flag to true, and re-writes metadata to the
    /// working copy.
    /// 
    /// This will create an original copy first, if one does not already exist.
    pub fn crop(&mut self, start: Duration, end: Duration) -> Result<()> {
        self.create_original_copy()?;

        // TODO: There are probably pure-Rust libraries for this, look into using those
        // TODO: should this be async like downloads are?
        println!("Starting FFMPEG...");

        let output = Command::new("ffmpeg")
            .arg("-ss")
            .arg((start.as_secs_f64()).to_string())
            .arg("-to")
            .arg((end.as_secs_f64()).to_string())
            .arg("-i")
            .arg(self.original_copy_path())
            .arg("-y")
            .arg("-acodec")
            .arg("copy")
            .arg(&self.path)
            .output()?;

        println!("FFMPEG is done!");

        // Check success
        output.status.exit_ok()?;

        self.metadata.is_cropped = true;
        self.metadata.write_into_file(&self.path)?;

        Ok(())
    }

    /// Modifies the working copy of this song to update its metadata to the current value of
    /// [`self.metadata`], as well as setting the [`SongMetadata.is_metadata_edited`] flag to true.
    /// 
    /// This will create an original copy first, if one does not already exist.
    pub fn user_edit_metadata(&mut self) -> Result<()> {
        self.create_original_copy()?;

        self.metadata.is_metadata_edited = true;
        self.metadata.write_into_file(&self.path)?;

        Ok(())
    }

    /// Deletes all copies of this song (working and original) from the library folder on disk.
    pub fn delete(&mut self) -> Result<()> {
        if self.original_copy_path().exists() {
            std::fs::remove_file(self.original_copy_path())?;
        }
        std::fs::remove_file(&self.path)?;

        Ok(())
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct SongMetadata {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub youtube_id: String,
    pub album_art: Option<Picture>,

    pub is_cropped: bool,
    pub is_metadata_edited: bool,
    pub download_unix_time: u64,
}

impl SongMetadata {
    fn get_album_art(tag: &Tag) -> Option<Picture> {
        tag.frames().find_map(|f|
            if let Some(picture) = f.content().picture() {
                if picture.picture_type == PictureType::CoverFront {
                    Some(picture.clone())
                } else {
                    None
                }
            } else {
                None
            }
        )
    }

    fn write_into_tag(&self, tag: &mut Tag) {
        // Unpacking here looks a bit weird, but it ensures that new fields will cause an error if
        // we forget to consider saving them
        let Self { title, artist, album, youtube_id, album_art, is_cropped, is_metadata_edited, download_unix_time } = self;

        tag.set_title(title.clone());
        tag.set_artist(artist.clone());
        tag.set_album(album.clone());
        if let Some(album_art) = album_art.clone() {
            tag.add_frame(album_art);
        }

        tag.write_custom::<YouTubeIdTag>(youtube_id.to_string());
        tag.write_custom::<DownloadTimeTag>(*download_unix_time);
        tag.write_custom::<CroppedTag>(*is_cropped);
        tag.write_custom::<MetadataEditedTag>(*is_metadata_edited);
    }

    pub(crate) fn write_into_file(&self, file: &Path) -> Result<()> {
        let mut tag = Tag::new();
        self.write_into_tag(&mut tag);
        Tag::write_to_path(&tag, file, id3::Version::Id3v23)?;
        Ok(())
    }
}
