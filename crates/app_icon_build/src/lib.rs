#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use std::{
    io,
    path::{Path, PathBuf},
};

use bevy_widgetry_diagnostics::{ErrorFrame, WidgetryFailure};
use std::{env::VarError, error::Error, fmt};

pub type IconBuildFailure = WidgetryFailure<IconBuildError>;

#[derive(Debug)]
pub enum IconBuildError {
    Environment {
        variable: &'static str,
        path: PathBuf,
        source: VarError,
    },
    InvalidPath {
        path: PathBuf,
    },
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Compile {
        path: PathBuf,
        source: io::Error,
    },
}
impl fmt::Display for IconBuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Environment { variable, path, .. } => write!(
                f,
                "icon build environment {variable} unavailable for {}",
                path.display()
            ),
            Self::InvalidPath { path } => write!(f, "icon path must be UTF-8: {path:?}"),
            Self::Read { path, .. } => write!(f, "cannot read icon {}", path.display()),
            Self::Compile { path, .. } => {
                write!(f, "icon resource compilation failed for {}", path.display())
            }
        }
    }
}
impl Error for IconBuildError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Environment { source, .. } => Some(source),
            Self::Read { source, .. } | Self::Compile { source, .. } => Some(source),
            Self::InvalidPath { .. } => None,
        }
    }
}

fn environment(variable: &'static str, path: &Path) -> Result<String, IconBuildFailure> {
    std::env::var(variable).map_err(|source| {
        WidgetryFailure::new(
            IconBuildError::Environment {
                variable,
                path: path.to_owned(),
                source,
            },
            ErrorFrame::new("icon.build.environment")
                .field("variable", variable)
                .field("path", path.display()),
        )
    })
}

pub fn set_exe_icon(path: impl AsRef<Path>) -> Result<(), IconBuildFailure> {
    let path = path.as_ref();
    let manifest_dir = environment("CARGO_MANIFEST_DIR", path)?;
    let path = resolve_icon_path(path, Path::new(&manifest_dir));
    println!("cargo:rerun-if-changed={}", path.display());
    let utf8 = path.to_str().ok_or_else(|| {
        WidgetryFailure::new(
            IconBuildError::InvalidPath { path: path.clone() },
            ErrorFrame::new("icon.build.path").field("path", format_args!("{path:?}")),
        )
    })?;
    std::fs::File::open(&path).map_err(|source| {
        WidgetryFailure::new(
            IconBuildError::Read {
                path: path.clone(),
                source,
            },
            ErrorFrame::new("icon.build.read").field("path", path.display()),
        )
    })?;
    // winresource reads these variables without a fallible API.
    // Validate Cargo's prerequisites before entering its resource constructor/compiler.
    for variable in [
        "CARGO_PKG_VERSION",
        "CARGO_PKG_NAME",
        "CARGO_PKG_VERSION_MAJOR",
        "CARGO_PKG_VERSION_MINOR",
        "CARGO_PKG_VERSION_PATCH",
        "CARGO_CFG_TARGET_ENV",
    ] {
        environment(variable, &path)?;
    }
    if std::env::var("CROSS_COMPILE").is_err() {
        environment("HOST", &path)?;
        environment("TARGET", &path)?;
    }
    winresource::WindowsResource::new()
        .set_icon(utf8)
        .compile()
        .map_err(|source| {
            WidgetryFailure::new(
                IconBuildError::Compile {
                    path: path.clone(),
                    source,
                },
                ErrorFrame::new("icon.build.compile").field("path", path.display()),
            )
        })
}

fn resolve_icon_path(path: &Path, manifest_dir: &Path) -> PathBuf {
    manifest_dir.join(path)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_relative_to_the_calling_package() {
        let manifest_dir = Path::new("C:/applications/example");
        assert_eq!(
            resolve_icon_path(Path::new("icons/app.ico"), manifest_dir),
            manifest_dir.join("icons/app.ico")
        );
        let absolute = Path::new("D:/icons/app.ico");
        assert_eq!(resolve_icon_path(absolute, manifest_dir), absolute);
    }

    #[test]
    fn missing_manifest_environment_returns_a_typed_error_without_panicking() {
        const CHILD: &str = "WIDGETRY_ICON_MISSING_MANIFEST_CHILD";
        if std::env::var_os(CHILD).is_some() {
            let failure = set_exe_icon("app.ico").unwrap_err();
            assert!(matches!(
                failure.kind(),
                IconBuildError::Environment {
                    variable: "CARGO_MANIFEST_DIR",
                    source: VarError::NotPresent,
                    ..
                }
            ));
            assert_eq!(
                failure.origin().backtrace().status(),
                std::backtrace::BacktraceStatus::Captured
            );
            return;
        }
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "tests::missing_manifest_environment_returns_a_typed_error_without_panicking",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .env_remove("CARGO_MANIFEST_DIR")
            .env_remove("RUST_BACKTRACE")
            .env_remove("RUST_LIB_BACKTRACE")
            .env_remove("BEVY_BACKTRACE")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn missing_host_or_target_returns_a_typed_failure_before_dependency_construction() {
        const CHILD: &str = "WIDGETRY_ICON_MISSING_TARGET_CHILD";
        if let Ok(variable) = std::env::var(CHILD) {
            // Dependency construction reads HOST/TARGET before parsing the icon bytes.
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
            let failure = set_exe_icon(&path).unwrap_err();
            assert!(
                matches!(failure.kind(), IconBuildError::Environment { variable: actual, source: VarError::NotPresent, path: actual_path } if *actual == variable && actual_path == &path)
            );
            return;
        }
        for missing in ["HOST", "TARGET"] {
            let mut command = std::process::Command::new(std::env::current_exe().unwrap());
            command.args(["--exact", "tests::missing_host_or_target_returns_a_typed_failure_before_dependency_construction", "--nocapture"])
                .env(CHILD, missing).env("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"))
                .env("CARGO_PKG_VERSION", "0.1.0").env("CARGO_PKG_NAME", "probe")
                .env("CARGO_PKG_VERSION_MAJOR", "0").env("CARGO_PKG_VERSION_MINOR", "1").env("CARGO_PKG_VERSION_PATCH", "0")
                .env("CARGO_CFG_TARGET_ENV", "msvc").env("HOST", "x86_64-pc-windows-msvc").env("TARGET", "x86_64-pc-windows-msvc")
                .env_remove(missing).env_remove("CROSS_COMPILE")
                .env_remove("RUST_BACKTRACE").env_remove("RUST_LIB_BACKTRACE").env_remove("BEVY_BACKTRACE");
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{missing}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[test]
    fn non_utf8_icon_keeps_the_original_windows_path() {
        use std::os::windows::ffi::OsStringExt;
        let path = PathBuf::from(std::ffi::OsString::from_wide(&[0xd800]));
        let failure = set_exe_icon(&path).unwrap_err();
        assert!(
            matches!(failure.kind(), IconBuildError::InvalidPath { path: actual } if actual.ends_with(&path))
        );
    }

    #[test]
    fn missing_icon_returns_not_found() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing-widgetry-icon.ico");
        let error = set_exe_icon(path).unwrap_err();
        assert!(
            matches!(error.kind(), IconBuildError::Read { source, .. } if source.kind() == io::ErrorKind::NotFound)
        );
        assert_eq!(
            error.origin().backtrace().status(),
            std::backtrace::BacktraceStatus::Captured
        );
    }
}
