use std::{sync::RwLockReadGuard, ops::Deref};

use async_process::{Command, Output};
use serde_json::Value;

use crate::library::{Library, SongMetadata, LibraryError};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct YouTubeDownload {
    pub id: String,
}

#[derive(Debug)]
pub enum DownloadError {
    IoError(std::io::Error),
    YouTubeDLNonZeroExit(Output),
    LibraryError(LibraryError),
}

impl YouTubeDownload {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn url(&self) -> String {
        format!("https://youtube.com/watch?v={}", self.id)
    }

    pub async fn download(&self, library: impl Deref<Target = Library>) -> Result<(), DownloadError> {
        println!("[Download] Starting...");

        // Might be a reference through a lock, if we don't drop it now we'll be holding it for ages
        let download_path = library.path.join(format!("{}.mp3", self.id));
        drop(library);
        
        // Ask youtube-dl to download this video
        let output = Command::new("youtube-dl")
            .arg("--print-json")
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--output")
            .arg(download_path.clone())
            .arg(self.url())
            .output()
            .await
            .map_err(|e| DownloadError::IoError(e))?;

        println!("[Download] Command complete");

        // Check success
        if !output.status.success() {
            return Err(DownloadError::YouTubeDLNonZeroExit(output))
        }

        println!("[Download] Command has zero exit status");

        // Build up metadata
        let metadata = Self::youtube_dl_output_to_metadata(output)
            .unwrap_or(SongMetadata {
                title: self.id.clone(),
                artist: "Unknown Artist".into(),
                album: "Unknown Album".into(),
                youtube_id: self.id.clone(),
            });

        println!("[Download] Build metadata object");

        // Write metadata into file
        metadata.write_into_file(&download_path).map_err(|e| DownloadError::LibraryError(e))?;

        println!("[Download] Written to file");

        Ok(())
    }

    fn youtube_dl_output_to_metadata(output: Output) -> Option<SongMetadata> {
        // First line of output is a JSON dump about the video (because we passed --print-json)
        let stdout_str = String::from_utf8(output.stdout).ok()?;
        let stdout_json: Value = serde_json::from_str(&stdout_str).ok()?;
        
        Some(SongMetadata {
            title: stdout_json["title"].as_str()?.into(),
            artist: stdout_json["uploader"].as_str()?.into(),
            album: "Unknown Album".into(),
            youtube_id: stdout_json["id"].as_str()?.into(),
        })
    }
}
