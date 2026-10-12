use crate::i18n::Language;
use bevy::prelude::*;
use bevy_widgetry::theme::WidgetryThemeMode;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;

#[derive(Resource, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AppConfig {
    pub(crate) window_width: u32,
    pub(crate) window_height: u32,
    pub(crate) scale_factor: f32,
    #[serde(with = "ThemeMode")]
    pub(crate) theme: WidgetryThemeMode,
    #[serde(with = "ConfigLanguage")]
    pub(crate) language: Language,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "WidgetryThemeMode")]
enum ThemeMode {
    Light,
    Dark,
    PinkDream,
    KamuriViolet,
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
            window_width: 1920,
            window_height: 1080,
            scale_factor: 1.25,
            theme: WidgetryThemeMode::Dark,
            language: Language::English,
        }
    }
}

impl AppConfig {
    fn validate(&self) -> io::Result<()> {
        if self.window_width == 0
            || self.window_height == 0
            || self.window_width > i32::MAX as u32
            || self.window_height > i32::MAX as u32
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "窗口宽高必须在 1..=i32::MAX 范围内",
            ));
        }
        if ![1.0, 1.25, 1.5, 2.0].contains(&self.scale_factor) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "scale_factor 必须为 1.0、1.25、1.5 或 2.0",
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
            warn!(path = %file.path.display(), %error, "保存 Gallery 配置失败，保留已应用的 UI 修改");
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

enum LoadDiagnostic {
    Missing,
    Read(io::Error),
    Parse(ron::error::SpannedError),
    Validate(io::Error),
    Create(io::Error),
}

pub(crate) struct LoadedConfig {
    pub(crate) config: AppConfig,
    pub(crate) file: ConfigFile,
    diagnostics: Vec<LoadDiagnostic>,
}

impl LoadedConfig {
    pub(crate) fn report(&self) {
        let path = self.file.path.display();
        for diagnostic in &self.diagnostics {
            match diagnostic {
                LoadDiagnostic::Missing => {
                    info!(%path, "Gallery 配置文件不存在，使用默认配置并尝试创建");
                }
                LoadDiagnostic::Read(error) => {
                    warn!(%path, %error, "读取 Gallery 配置失败，整体使用默认配置并保留原文件");
                }
                LoadDiagnostic::Parse(error) => {
                    error!(%path, %error, "解析 Gallery 配置失败，整体使用默认配置并保留原文件");
                }
                LoadDiagnostic::Validate(error) => {
                    warn!(%path, %error, "校验 Gallery 配置失败，整体使用默认配置并保留原文件");
                }
                LoadDiagnostic::Create(error) => {
                    warn!(%path, %error, "创建 Gallery 默认配置失败，继续使用内存默认配置");
                }
            }
        }
    }
}

pub(crate) fn load(path: PathBuf) -> LoadedConfig {
    let file = ConfigFile { path };
    let mut diagnostics = Vec::new();
    let config = match fs::read_to_string(&file.path) {
        Ok(content) => match ron::from_str::<AppConfig>(&content) {
            Ok(config) => match config.validate() {
                Ok(()) => config,
                Err(error) => {
                    diagnostics.push(LoadDiagnostic::Validate(error));
                    AppConfig::default()
                }
            },
            Err(error) => {
                diagnostics.push(LoadDiagnostic::Parse(error));
                AppConfig::default()
            }
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            diagnostics.push(LoadDiagnostic::Missing);
            let config = AppConfig::default();
            if let Err(error) = file.create_default(&config) {
                diagnostics.push(LoadDiagnostic::Create(error));
            }
            config
        }
        Err(error) => {
            diagnostics.push(LoadDiagnostic::Read(error));
            AppConfig::default()
        }
    };
    LoadedConfig {
        config,
        file,
        diagnostics,
    }
}
