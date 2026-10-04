use crate::{
    WidgetryFileDialogCandidate, WidgetryFileDialogEntryData, WidgetryFileDialogEntryKind,
    WidgetryFileDialogResult, WidgetryFileDialogStorageSnapshot,
};
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::io;
#[cfg(windows)]
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Resource, Clone)]
pub struct WidgetryFileDialogBackend(pub Arc<dyn WidgetryFileDialogFileSystem>);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetryFileDialogLocation {
    pub name: String,
    pub path: PathBuf,
}

pub struct WidgetryFileDialogNativeFileSystem;

pub trait WidgetryFileDialogFileSystem: Send + Sync + 'static {
    fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf>;

    fn read_directory(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>>;

    fn validate(&self, candidate: &WidgetryFileDialogCandidate) -> io::Result<bool>;

    fn create_directory(&self, path: &Path) -> io::Result<()>;

    fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>>;

    fn load_preferences(
        &self,
        path: &Path,
    ) -> io::Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>> {
        crate::persistence::load(path)
    }

    fn save_preferences(
        &self,
        path: &Path,
        scopes: &BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
    ) -> io::Result<()> {
        crate::persistence::save(path, scopes)
    }
}

impl Default for WidgetryFileDialogBackend {
    fn default() -> Self {
        Self(Arc::new(WidgetryFileDialogNativeFileSystem))
    }
}

impl WidgetryFileDialogFileSystem for WidgetryFileDialogNativeFileSystem {
    fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf> {
        let path = match path {
            Some(path) => path.to_owned(),
            None => std::env::current_dir()?,
        };
        let resolved = fs::canonicalize(path)?;
        if !fs::metadata(&resolved)?.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "requested path is not a directory",
            ));
        }
        Ok(resolved)
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>> {
        Ok(Box::new(fs::read_dir(path)?.map(|entry| {
            let entry = entry?;
            let path = entry.path();
            let name = entry.file_name();
            let metadata = entry.metadata()?;
            let file_type = metadata.file_type();
            let kind = if file_type.is_symlink() {
                match fs::metadata(&path) {
                    Ok(target) if target.is_dir() => WidgetryFileDialogEntryKind::SymlinkDirectory,
                    Ok(target) if target.is_file() => WidgetryFileDialogEntryKind::SymlinkFile,
                    Ok(_) => WidgetryFileDialogEntryKind::Unknown,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        WidgetryFileDialogEntryKind::DanglingSymlink
                    }
                    Err(error) => return Err(error),
                }
            } else if file_type.is_dir() {
                WidgetryFileDialogEntryKind::Directory
            } else if file_type.is_file() {
                WidgetryFileDialogEntryKind::File
            } else {
                WidgetryFileDialogEntryKind::Unknown
            };
            #[cfg(windows)]
            let (hidden, system) = (
                Some(metadata.file_attributes() & 2 != 0),
                Some(metadata.file_attributes() & 4 != 0),
            );
            #[cfg(not(windows))]
            let (hidden, system) = (Some(name.as_encoded_bytes().starts_with(b".")), Some(false));
            Ok(WidgetryFileDialogEntryData {
                path,
                name,
                kind,
                size: Some(metadata.len()),
                modified: Some(metadata.modified()?),
                hidden,
                system,
            })
        })))
    }

    fn validate(&self, candidate: &WidgetryFileDialogCandidate) -> io::Result<bool> {
        match candidate.result() {
            WidgetryFileDialogResult::SavePath(path) => {
                let parent = path
                    .parent()
                    .ok_or_else(|| io::Error::other("save path has no parent"))?;
                if !fs::metadata(parent)?.is_dir() {
                    return Err(io::Error::other("save parent is not a directory"));
                }
                match fs::symlink_metadata(path) {
                    Ok(_) => {
                        if !fs::metadata(path)?.is_file() {
                            return Err(io::Error::other("save target is not a file"));
                        }
                        Ok(true)
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                    Err(error) => Err(error),
                }
            }
            WidgetryFileDialogResult::File(path) => validate_path(path, false),
            WidgetryFileDialogResult::Directory(path) => validate_path(path, true),
            WidgetryFileDialogResult::Files(paths)
            | WidgetryFileDialogResult::Directories(paths) => {
                let directory =
                    matches!(candidate.result(), WidgetryFileDialogResult::Directories(_));
                for path in paths.iter() {
                    validate_path(path, directory)?;
                }
                Ok(true)
            }
            WidgetryFileDialogResult::Cancelled => {
                Err(io::Error::other("cancelled result cannot be validated"))
            }
        }
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path)
    }

    fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
        let user = directories::UserDirs::new()
            .ok_or_else(|| io::Error::other("user directories unavailable"))?;
        let mut locations = vec![WidgetryFileDialogLocation {
            name: "Home".into(),
            path: user.home_dir().to_owned(),
        }];
        for (name, path) in [
            ("Documents", user.document_dir()),
            ("Downloads", user.download_dir()),
            ("Desktop", user.desktop_dir()),
        ] {
            if let Some(path) = path {
                locations.push(WidgetryFileDialogLocation {
                    name: name.into(),
                    path: path.to_owned(),
                });
            }
        }
        #[cfg(windows)]
        for letter in b'A'..=b'Z' {
            let path = PathBuf::from(format!("{}:\\", char::from(letter)));
            match fs::metadata(&path) {
                Ok(metadata) if metadata.is_dir() => locations.push(WidgetryFileDialogLocation {
                    name: format!("{}:", char::from(letter)),
                    path,
                }),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(locations)
    }
}

fn validate_path(path: &Path, directory: bool) -> io::Result<bool> {
    let metadata = fs::metadata(path)?;
    if (directory && metadata.is_dir()) || (!directory && metadata.is_file()) {
        Ok(true)
    } else {
        Err(io::Error::other("candidate kind changed"))
    }
}

#[cfg(test)]
// 测试断言用于校验 filesystem 的真实结果，不适用生产 macro 禁令。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! filesystem state 为存在/消失、file/directory 与可访问/错误。
    //! stimuli 为 resolve、streaming enumeration、create 和 validation。
    //! invariant 为原始 path 保留、metadata 缓存、错误明确返回。
    use super::*;
    use std::fs;

    #[test]
    fn native_directory_stream_preserves_metadata_and_reports_invalid_path() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        fs::write(temporary.path().join("a.txt"), b"hello")?;
        let backend = WidgetryFileDialogNativeFileSystem;
        let resolved = backend.resolve_directory(Some(temporary.path()))?;
        let entries = backend
            .read_directory(&resolved)?
            .collect::<io::Result<Vec<_>>>()?;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].path, resolved.join("a.txt"));
        assert_eq!(entries[0].size, Some(5));
        assert!(entries[0].modified.is_some());
        backend.create_directory(&resolved.join("folder"))?;
        assert!(resolved.join("folder").is_dir());
        assert!(
            backend
                .resolve_directory(Some(&resolved.join("missing")))
                .is_err()
        );
        Ok(())
    }
}
