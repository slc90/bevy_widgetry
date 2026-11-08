const DEFAULT_SCALE_FACTOR: f32 = 1.25;
const ALLOWED_SCALE_FACTORS: [f32; 4] = [1.0, 1.25, 1.5, 2.0];
const DEFAULT_BRP_PORT: u16 = 15702;
const BRP_PORT_RANGE: std::ops::RangeInclusive<u16> = 1024..=65534;
const RENDER_WORLD_BRP_PORT: u16 = 15703;

use crate::theme::GalleryTheme;
use crate::{
    diagnostics::{self, GalleryError},
    i18n::Language,
};
use bevy::prelude::*;
use bevy_widgetry::diagnostics::ErrorFrame;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Resource, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppConfig {
    pub(crate) scale_factor: f32,
    pub(crate) theme: GalleryTheme,
    #[serde(with = "ConfigLanguage")]
    pub(crate) language: Language,
    pub(crate) brp_enabled: bool,
    pub(crate) brp_port: u16,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Language")]
enum ConfigLanguage {
    English,
    SimplifiedChinese,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            scale_factor: DEFAULT_SCALE_FACTOR,
            theme: GalleryTheme::Dark,
            language: Language::English,
            brp_enabled: true,
            brp_port: DEFAULT_BRP_PORT,
        }
    }
}

impl AppConfig {
    fn validate(&self) -> io::Result<()> {
        if !ALLOWED_SCALE_FACTORS.contains(&self.scale_factor) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "scale_factor 必须为 1.0、1.25、1.5 或 2.0",
            ));
        }
        if !BRP_PORT_RANGE.contains(&self.brp_port) || self.brp_port == RENDER_WORLD_BRP_PORT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "brp_port 必须为 1024..=65534，且不能占用 Render World 的 15703",
            ));
        }
        Ok(())
    }

    fn serialized(&self) -> io::Result<String> {
        self.validate()?;
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default())
            .map_err(io::Error::other)
    }

    pub(crate) fn save(&self, file: &ConfigFile) {
        let result = (|| {
            let content = self.serialized()?;
            file.create_directory()?;
            fs::write(&file.path, content)
        })();
        if let Err(error) = result {
            handled("save", &file.path, error, "保留已应用的 UI 修改");
        }
    }
}

#[derive(Resource)]
pub(crate) struct ConfigFile {
    path: PathBuf,
}

impl ConfigFile {
    fn create_directory(&self) -> io::Result<()> {
        let parent = self.path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Gallery 配置路径缺少父目录")
        })?;
        fs::create_dir_all(parent)
    }

    fn create_default(&self, config: &AppConfig) -> io::Result<()> {
        let content = config.serialized()?;
        self.create_directory()?;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&self.path)?;
        file.write_all(content.as_bytes())
    }
}

pub(crate) struct LoadedConfig {
    pub(crate) config: AppConfig,
    pub(crate) file: ConfigFile,
}

#[track_caller]
fn handled(
    action: &'static str,
    path: &std::path::Path,
    source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    outcome: &str,
) {
    let failure = diagnostics::failure(
        GalleryError::Configuration {
            action,
            path: path.to_owned(),
            source: source.into(),
        },
        ErrorFrame::new("gallery.config")
            .field("action", action)
            .field("path", path.display()),
    );
    failure.record_handled("采用 Gallery 配置降级策略", outcome);
}

pub(crate) fn load(path: PathBuf) -> LoadedConfig {
    let file = ConfigFile { path };
    let config = match fs::read_to_string(&file.path) {
        Ok(content) => match ron::from_str::<AppConfig>(&content) {
            Ok(config) => match config.validate() {
                Ok(()) => config,
                Err(error) => {
                    handled("validate", &file.path, error, "使用默认配置并保留原文件");
                    AppConfig::default()
                }
            },
            Err(error) => {
                handled("parse", &file.path, error, "使用默认配置并保留原文件");
                AppConfig::default()
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            debug!(path = %file.path.display(), "Gallery 配置文件不存在，使用默认配置并尝试创建");
            let config = AppConfig::default();
            if let Err(error) = file.create_default(&config) {
                handled("create", &file.path, error, "继续使用内存默认配置");
            }
            config
        }
        Err(error) => {
            handled("read", &file.path, error, "使用默认配置并保留原文件");
            AppConfig::default()
        }
    };
    LoadedConfig { config, file }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

    #[test]
    fn malformed_configuration_is_preserved_and_fallback_is_explicitly_handled() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.ron");
        fs::write(&path, "malformed").unwrap();
        let logs = LogCapture::with_targets(["widget_gallery", "bevy_widgetry"]);
        let loaded = logs.run(|| load(path.clone()));
        assert_eq!(
            loaded.config.scale_factor,
            AppConfig::default().scale_factor
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "malformed");
        let records = logs.records();
        assert_eq!(
            records
                .iter()
                .filter(|event| event.phase() == Some(DiagnosticPhase::Source))
                .count(),
            1
        );
        assert_eq!(
            records
                .iter()
                .filter(|event| event.phase() == Some(DiagnosticPhase::Handled))
                .count(),
            1
        );
        assert!(
            records
                .iter()
                .all(|event| !event.fields.contains_key("backtrace"))
        );
    }

    #[test]
    fn save_failure_preserves_the_applied_configuration_and_reports_its_path() {
        let directory = tempfile::tempdir().unwrap();
        let occupied = directory.path().join("occupied");
        fs::write(&occupied, "keep").unwrap();
        let file = ConfigFile {
            path: occupied.join("config.ron"),
        };
        let config = AppConfig {
            scale_factor: 2.0,
            ..default()
        };
        let logs = LogCapture::with_targets(["widget_gallery", "bevy_widgetry"]);
        logs.run(|| config.save(&file));
        assert_eq!(config.scale_factor, 2.0);
        assert_eq!(fs::read_to_string(occupied).unwrap(), "keep");
        let source = logs
            .records()
            .into_iter()
            .find(|event| event.phase() == Some(DiagnosticPhase::Source))
            .unwrap();
        assert!(source.fields["error"].contains(&file.path.display().to_string()));
        assert!(
            logs.records()
                .iter()
                .any(|event| event.phase() == Some(DiagnosticPhase::Handled))
        );
    }
}
