use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub library_path: PathBuf,
}

impl Settings {
    pub fn settings_dir() -> PathBuf {
        dirs::config_dir().expect("unknown OS").join("CrossPlay")
    }

    pub fn settings_path() -> PathBuf {
        Self::settings_dir().join("settings.json")
    }

    /// Loads the application settings, or creates them from defaults if they do not exist.
    pub fn load() -> Result<Self> {
        let path = Self::settings_path();
        if !path.exists() {
            Settings::default().save()?;
        }

        let settings_contents = std::fs::read_to_string(path)?;
        Ok(serde_json::from_str(&settings_contents)?)
    }

    /// Saves the application settings.
    pub fn save(&self) -> Result<()> {
        // Ensure settings dir exists
        if !Self::settings_dir().exists() {
            std::fs::create_dir(Self::settings_dir())?;
        }

        // Ensure library dir exists
        if !self.library_path.exists() {
            std::fs::create_dir(&self.library_path)?;
        }

        let json = serde_json::to_string(self)?;
        std::fs::write(Self::settings_path(), json)?;

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            library_path: dirs::audio_dir().expect("unknown OS").join("CrossPlay")
        }
    }
}
