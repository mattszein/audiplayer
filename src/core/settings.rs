use config::{Config, ConfigError, File, FileFormat};
use serde::Deserialize;

const DEFAULT_CONFIG: &str = include_str!("../../config/default.toml");

#[derive(Debug, Deserialize, Clone)]
pub struct ThemeSettings {
    pub name: String,
    pub mode: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PlaybackSettings {
    pub volume: u8,
    pub autoplay: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProvidersSettings {
    pub default: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GeneralSettings {
    pub log_file: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub theme: ThemeSettings,
    pub playback: PlaybackSettings,
    pub providers: ProvidersSettings,
    pub general: GeneralSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let config = Config::builder()
            .add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Toml))
            .add_source(
                File::with_name(
                    &Settings::user_config_path().unwrap_or_default()
                )
                .required(false),
            )
            .build()?;

        config.try_deserialize()
    }

    pub fn config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("audiplayer").join("settings.toml"))
    }

    fn user_config_path() -> Option<String> {
        Self::config_path().map(|p| p.to_string_lossy().into_owned())
    }
}
