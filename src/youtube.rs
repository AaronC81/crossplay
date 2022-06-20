use std::{sync::Arc, ops::Deref, io::{Cursor, BufReader}, path::{PathBuf, Path}, fs::File};

use async_process::{Command, Output};
use id3::frame::Picture;
use image::{ImageBuffer, ImageError, ImageFormat};
use serde_json::Value;

use crate::library::{Library, SongMetadata, LibraryError};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct YouTubeDownload {
    pub id: String,
}

#[derive(Clone, Debug)]
pub enum DownloadError {
    IoError(Arc<std::io::Error>),
    YouTubeDLNonZeroExit(Output),
    DownloadMissing,
    ThumbnailMissing,
    LibraryError(LibraryError),
    ImageError(Arc<ImageError>),
}

impl YouTubeDownload {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn url(&self) -> String {
        format!("https://youtube.com/watch?v={}", self.id)
    }

    pub async fn download(&self, library_path: &Path) -> Result<(), DownloadError> {
        println!("[Download] Starting...");

        let download_path = library_path.join(format!("{}.%(ext)s", self.id));
        
        // Ask youtube-dl to download this video
        let output = Command::new("youtube-dl")
            .arg("--print-json")
            .arg("--extract-audio")
            .arg("--write-thumbnail")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--output")
            .arg(download_path.clone())
            .arg(self.url())
            .output()
            .await
            .map_err(|e| DownloadError::IoError(Arc::new(e)))?;

        println!("[Download] Command complete");

        // Check success
        if !output.status.success() {
            return Err(DownloadError::YouTubeDLNonZeroExit(output))
        }

        println!("[Download] Command has zero exit status");

        // The download path we were working with up to this point is templated for youtube-dl with
        // an unknown extension. Make sure we actually downloaded an MP3
        let download_path = library_path.join(format!("{}.mp3", self.id));
        if !download_path.exists() {
            return Err(DownloadError::DownloadMissing)
        }

        // We should've downloaded a thumbnail too, figure out where that is
        let thumbnail_possible_extensions = ["jpg", "jpeg", "webp", "png"];
        let thumbnail_path = thumbnail_possible_extensions
            .iter()
            .find_map(|ext| {
                let path = library_path.join(format!("{}.{}", self.id, ext));
                if path.exists() {
                    Some(path)
                } else {
                    None
                }
            })
            .ok_or(DownloadError::ThumbnailMissing)?;

        // Convert to JPEG
        // Originally, this tried to be clever and only convert if the image was a WEBP - but
        // YouTube sometimes lies and sends us WEBPs with a .jpg extension
        // https://github.com/ytdl-org/youtube-dl/issues/29754 
        // Using image::io::Reader rather than image::open lets us use `with_guessed_format`, which
        // guesses using content instead of path, circumventing this
        let reader = BufReader::new(File::open(&thumbnail_path).map_err(|e| DownloadError::IoError(Arc::new(e)))?);
        let loaded_file = image::io::Reader::new(reader)
            .with_guessed_format()
            .map_err(|e| DownloadError::IoError(Arc::new(e)))?
            .decode()
            .map_err(|e| DownloadError::ImageError(Arc::new(e)))?;
        let mut jpeg_bytes = Cursor::new(vec![]);
        loaded_file.write_to(&mut jpeg_bytes, ImageFormat::Jpeg).map_err(|e| DownloadError::ImageError(Arc::new(e)))?;
        let thumbnail_data = jpeg_bytes.into_inner();

        // Convert thumbnail into an ID3 picture
        let thumbnail_picture = Picture {
            mime_type: "image/jpeg".to_string(),
            picture_type: id3::frame::PictureType::CoverFront,
            description: "Cover".to_string(),
            data: thumbnail_data,
        };

        // Delete thumbnail file, since it's now encoded into ID3
        std::fs::remove_file(thumbnail_path).map_err(|e| DownloadError::IoError(Arc::new(e)))?;
            
        // Build up metadata
        let metadata = Self::youtube_dl_output_to_metadata(output, thumbnail_picture)
            .unwrap_or(SongMetadata {
                title: self.id.clone(),
                artist: "Unknown Artist".into(),
                album: "Unknown Album".into(),
                youtube_id: self.id.clone(),
                album_art: None,
                is_cropped: false,
                is_metadata_edited: false,
            });

        println!("[Download] Build metadata object");

        // Write metadata into file
        metadata.write_into_file(&download_path).map_err(|e| DownloadError::LibraryError(e))?;

        println!("[Download] Written to file");

        Ok(())
    }

    fn youtube_dl_output_to_metadata(output: Output, album_art: Picture) -> Option<SongMetadata> {
        // First line of output is a JSON dump about the video (because we passed --print-json)
        let stdout_str = String::from_utf8(output.stdout).ok()?;
        let stdout_json: Value = serde_json::from_str(&stdout_str).ok()?;
        
        Some(SongMetadata {
            title: stdout_json["title"].as_str()?.into(),
            artist: stdout_json["uploader"].as_str()?.into(),
            album: "Unknown Album".into(),
            youtube_id: stdout_json["id"].as_str()?.into(),
            album_art: Some(album_art),
            is_cropped: false,
            is_metadata_edited: false,
        })
    }
}
