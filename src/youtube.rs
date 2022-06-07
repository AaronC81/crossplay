use async_process::{Command, Output};

use crate::library::Library;

#[derive(Debug)]
pub struct YouTubeDownload {
    pub id: String,
}

#[derive(Debug)]
pub enum DownloadError {
    IoError(std::io::Error),
    YouTubeDLNonZeroExit(Output),
}

impl YouTubeDownload {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn url(&self) -> String {
        format!("https://youtube.com/watch?v={}", self.id)
    }

    pub async fn download(&self, library: &Library) -> Result<(), DownloadError> {
        let output = Command::new("youtube-dl")
            .arg("--extract-audio")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--output")
            .arg(library.path.join(format!("{}.%(ext)s", self.id)))
            .arg(self.url())
            .output()
            .await
            .map_err(|e| DownloadError::IoError(e))?;

        if !output.status.success() {
            return Err(DownloadError::YouTubeDLNonZeroExit(output))
        }

        Ok(())
    }
}
