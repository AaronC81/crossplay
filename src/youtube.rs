use std::{sync::{Arc, RwLock}, io::{Cursor, BufReader}, path::{PathBuf, Path}, fs::File, time::{SystemTime, UNIX_EPOCH}};

use anyhow::{Result, anyhow};
use async_process::{Command, Stdio};
use id3::frame::Picture;
use image::ImageFormat;
use regex::Regex;
use serde_json::Value;
use iced::futures::{io::BufReader as AsyncBufReader, AsyncBufReadExt, StreamExt};

use crate::library::SongMetadata;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct YouTubeDownload {
    pub id: String,
}

pub struct YouTubeDownloadProgress {
    pub progress: f32,
    pub metadata: Option<SongMetadata>,
}

impl YouTubeDownloadProgress {
    pub fn new() -> Self {
        Self { progress: 0.0, metadata: None }
    }
}

impl Default for YouTubeDownloadProgress {
    fn default() -> Self { Self::new() }
}

impl YouTubeDownload {
    pub fn new(id: impl Into<String>) -> Self {
        Self { id: id.into() }
    }

    pub fn url(&self) -> String {
        format!("https://youtube.com/watch?v={}", self.id)
    }

    pub async fn download(&self, library_path: &Path, progress: Arc<RwLock<YouTubeDownloadProgress>>) -> Result<()> {
        println!("[Download] Starting...");

        // Set up initial progress, just in case we were passed a dirty object
        // Note: The blocks dispersed throughout this function around usages of `progress`, like
        // this one, are to stop the compiler getting angry about passing RwLocks across thread
        // boundaries (even though we aren't because of `drop`s)
        {
            let mut progress_writer = progress.write().unwrap();
            *progress_writer = YouTubeDownloadProgress::new();
            drop(progress_writer);
        }

        let download_path = library_path.join(format!("{}.%(ext)s", self.id));
        
        // Ask youtube-dl to download this video
        let mut process = Command::new("youtube-dl")
            .arg("--write-info-json")
            .arg("--extract-audio")
            .arg("--write-thumbnail")
            .arg("--newline")
            .arg("--audio-format")
            .arg("mp3")
            .arg("--output")
            .arg(download_path.clone())
            .arg(self.url())
            .stdout(Stdio::piped())
            .spawn()?;

        let mut line_reader = AsyncBufReader::new(process.stdout.take().unwrap()).lines();
        let json_file_regex = Regex::new("Writing video description metadata as JSON to: (.+)$").unwrap();
        let progress_regex = Regex::new(r"\[download\]\s*(\d+\.\d+)%").unwrap();
        while let Some(line) = line_reader.next().await {
            let line = line?;

            // Look for the line which tells us where our metadata file is
            if let Some(captures) = json_file_regex.captures(&line) {
                // youtube-dl says it written the file, but that's not a guarantee, sometimes it
                // can take a little while (presumably due to disk flusing)
                // Wait for it to exist
                // TODO: delay between checks, maybe with timeout
                let json_file = captures.get(1).unwrap().as_str();
                while !PathBuf::from(json_file).exists() {}

                let contents = std::fs::read_to_string(json_file)?;
                
                // Convert into metadata
                {
                    let mut progress_writer = progress.write().unwrap();
                    progress_writer.metadata = Self::youtube_dl_output_to_metadata(contents);
                    drop(progress_writer);
                }

                // Delete file - we've got what we need
                std::fs::remove_file(json_file)?;
            }

            // Also look for progress updates
            if let Some(captures) = progress_regex.captures(&line) {
                let percentage = captures.get(1).unwrap().as_str();

                {
                    let mut progress_writer = progress.write().unwrap();
                    progress_writer.progress = percentage.parse().unwrap();
                    drop(progress_writer);
                }
            }
        }

        // If we never got any metadata, initialise it
        let mut metadata;
        {
            let progress_reader = progress.read().unwrap();
            metadata = progress_reader.metadata.clone().unwrap_or_else(||
                SongMetadata {
                    title: self.id.clone(),
                    artist: "Unknown Artist".into(),
                    album: "Unknown Album".into(),
                    youtube_id: self.id.clone(),
                    album_art: None,
                    is_cropped: false,
                    is_metadata_edited: false,
                    download_unix_time: unix_time_now(),
                }
            );
            drop(progress_reader);
            drop(progress);
        }

        // Check success
        let status = process.status().await?;
        status.exit_ok()?;

        println!("[Download] Command has zero exit status");

        // The download path we were working with up to this point is templated for youtube-dl with
        // an unknown extension. Make sure we actually downloaded an MP3
        let download_path = library_path.join(format!("{}.mp3", self.id));
        if !download_path.exists() {
            return Err(anyhow!("Downloaded MP3 could not be located."));
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
            .ok_or_else(|| anyhow!("Downloaded thumbnail could not be located."))?;

        // Convert to JPEG
        // Originally, this tried to be clever and only convert if the image was a WEBP - but
        // YouTube sometimes lies and sends us WEBPs with a .jpg extension
        // https://github.com/ytdl-org/youtube-dl/issues/29754 
        // Using image::io::Reader rather than image::open lets us use `with_guessed_format`, which
        // guesses using content instead of path, circumventing this
        let reader = BufReader::new(File::open(&thumbnail_path)?);
        let loaded_file = image::io::Reader::new(reader)
            .with_guessed_format()?
            .decode()?;
        let mut jpeg_bytes = Cursor::new(vec![]);
        loaded_file.write_to(&mut jpeg_bytes, ImageFormat::Jpeg)?;
        let thumbnail_data = jpeg_bytes.into_inner();

        // Convert thumbnail into an ID3 picture
        let thumbnail_picture = Picture {
            mime_type: "image/jpeg".to_string(),
            picture_type: id3::frame::PictureType::CoverFront,
            description: "Cover".to_string(),
            data: thumbnail_data,
        };

        // Delete thumbnail file, since it's now encoded into ID3
        std::fs::remove_file(thumbnail_path)?;
            
        // Assign thumbnail
        metadata.album_art = Some(thumbnail_picture); 

        println!("[Download] Build metadata object");

        // Write metadata into file
        metadata.write_into_file(&download_path)?;

        println!("[Download] Written to file");

        Ok(())
    }

    fn youtube_dl_output_to_metadata(string: String) -> Option<SongMetadata> {
        let stdout_json: Value = serde_json::from_str(&string).ok()?;
        
        Some(SongMetadata {
            title: stdout_json["title"].as_str()?.into(),
            artist: stdout_json["uploader"].as_str()?.into(),
            album: "Unknown Album".into(),
            youtube_id: stdout_json["id"].as_str()?.into(),
            album_art: None,
            is_cropped: false,
            is_metadata_edited: false,
            download_unix_time: unix_time_now(),
        })
    }
}

/// Attempts to extract a YouTube video ID from the given string. This is done by looking for the
/// following URL patterns:
///   - youtube.com/watch?v=...
///   - youtu.be/...
/// If neither of these match, then the original string will be returned. As such, there is no
/// guarantee that the video ID will be valid or in the correct format.
pub fn extract_video_id(string: &str) -> &str {
    let long_url_regex = Regex::new(r"youtube.com/watch\?v=([^&]+)&?").unwrap();
    let short_url_regex = Regex::new(r"youtu.be/([^&]+)&?").unwrap();

    if let Some(c) = long_url_regex.captures(string) {
        return c.get(1).unwrap().as_str();
    }

    if let Some(c) = short_url_regex.captures(string) {
        return c.get(1).unwrap().as_str();
    }

    string
}

fn unix_time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}
