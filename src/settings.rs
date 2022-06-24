use std::path::PathBuf;

use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum SortBy {
    Title,
    Artist,
    Album,
    Downloaded,
}

#[derive(Debug, Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum SortDirection {
    Normal,
    Reverse,
}

impl SortDirection {
    pub fn reverse(self) -> SortDirection {
        match self {
            SortDirection::Normal => SortDirection::Reverse,
            SortDirection::Reverse => SortDirection::Normal,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default = "Settings::default_library_path")]
    pub library_path: PathBuf,

    #[serde(default = "Settings::default_sort_by")]
    pub sort_by: SortBy,

    #[serde(default = "Settings::default_sort_direction")]
    pub sort_direction: SortDirection,
}

impl Settings {
    pub fn settings_dir() -> PathBuf {
        dirs::config_dir().expect("unknown OS").join("CrossPlay")
    }

    pub fn settings_path() -> PathBuf {
        Self::settings_dir().join("settings.json")
    }

    pub fn default_library_path() -> PathBuf {
        dirs::audio_dir().expect("unknown OS").join("CrossPlay")
    }
    pub fn default_sort_by() -> SortBy { SortBy::Downloaded }
    pub fn default_sort_direction() -> SortDirection { SortDirection::Normal }

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
            library_path: Self::default_library_path(),
            sort_by: Self::default_sort_by(),
            sort_direction: Self::default_sort_direction(),
        }
    }
}
