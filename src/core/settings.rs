use config::{Config, ConfigError, File, FileFormat};
use serde::Deserialize;

const DEFAULT_CONFIG: &str = project_file!("config/default.toml");

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

    pub fn ensure_config_file() -> String {
        let Some(path) = Self::config_path() else {
            return "Could not determine config directory".to_string();
        };

        if path.exists() {
            return format!("Config: {}", path.display());
        }

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return format!("Failed to create config dir: {}", e);
            }
        }

        let example = project_file!("config/example.toml");
        match std::fs::write(&path, example) {
            Ok(()) => format!("Created config: {}", path.display()),
            Err(e) => format!("Failed to create config file: {}", e),
        }
    }

    fn config_path() -> Option<std::path::PathBuf> {
        dirs::config_dir().map(|p| p.join("audiplayer").join("settings.toml"))
    }

    fn user_config_path() -> Option<String> {
        Self::config_path().map(|p| p.to_string_lossy().into_owned())
    }
}
