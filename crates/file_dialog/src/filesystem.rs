use crate::FileDialogRequestOperation;
use crate::error::{native_directory_io, native_request_io};
use crate::{FileDialogDirectoryContext, FileDialogDirectoryOperation};
use crate::{
    WidgetryFileDialogCandidate, WidgetryFileDialogEntryData, WidgetryFileDialogEntryKind,
    WidgetryFileDialogResult, WidgetryFileDialogStorageSnapshot,
};
use bevy::prelude::*;
use std::collections::BTreeMap;
use std::fs;
use std::io;
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

fn native_context(
    operation: FileDialogDirectoryOperation,
    directory: Option<PathBuf>,
    entry: Option<PathBuf>,
    ordinal: Option<usize>,
) -> FileDialogDirectoryContext {
    FileDialogDirectoryContext {
        operation,
        directory,
        entry,
        ordinal,
        root: None,
        token: None,
    }
}

impl WidgetryFileDialogFileSystem for WidgetryFileDialogNativeFileSystem {
    fn resolve_directory(&self, path: Option<&Path>) -> io::Result<PathBuf> {
        let path = match path {
            Some(path) => path.to_owned(),
            None => std::env::current_dir().map_err(|error| {
                native_directory_io(
                    native_context(
                        FileDialogDirectoryOperation::CurrentDirectory,
                        None,
                        None,
                        None,
                    ),
                    error,
                )
            })?,
        };
        let resolved = fs::canonicalize(&path).map_err(|error| {
            native_directory_io(
                native_context(
                    FileDialogDirectoryOperation::Canonicalize,
                    Some(path.clone()),
                    None,
                    None,
                ),
                error,
            )
        })?;
        if !fs::metadata(&resolved)
            .map_err(|error| {
                native_directory_io(
                    native_context(
                        FileDialogDirectoryOperation::DirectoryMetadata,
                        Some(resolved.clone()),
                        None,
                        None,
                    ),
                    error,
                )
            })?
            .is_dir()
        {
            return Err(native_directory_io(
                native_context(
                    FileDialogDirectoryOperation::DirectoryMetadata,
                    Some(resolved),
                    None,
                    None,
                ),
                io::Error::new(
                    io::ErrorKind::NotADirectory,
                    "requested path is not a directory",
                ),
            ));
        }
        Ok(resolved)
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> io::Result<Box<dyn Iterator<Item = io::Result<WidgetryFileDialogEntryData>> + Send>> {
        let directory = path.to_owned();
        let entries = fs::read_dir(path).map_err(|error| {
            native_directory_io(
                native_context(
                    FileDialogDirectoryOperation::ReadDirectory,
                    Some(directory.clone()),
                    None,
                    None,
                ),
                error,
            )
        })?;
        Ok(Box::new(entries.enumerate().map(
            move |(ordinal, entry)| {
                let entry = entry.map_err(|error| {
                    native_directory_io(
                        native_context(
                            FileDialogDirectoryOperation::ReadEntry,
                            Some(directory.clone()),
                            None,
                            Some(ordinal),
                        ),
                        error,
                    )
                })?;
                let path = entry.path();
                let name = entry.file_name();
                let metadata = entry.metadata().map_err(|error| {
                    native_directory_io(
                        native_context(
                            FileDialogDirectoryOperation::EntryMetadata,
                            Some(directory.clone()),
                            Some(path.clone()),
                            Some(ordinal),
                        ),
                        error,
                    )
                })?;
                let file_type = metadata.file_type();
                let kind = if file_type.is_symlink() {
                    match fs::metadata(&path) {
                        Ok(target) if target.is_dir() => {
                            WidgetryFileDialogEntryKind::SymlinkDirectory
                        }
                        Ok(target) if target.is_file() => WidgetryFileDialogEntryKind::SymlinkFile,
                        Ok(_) => WidgetryFileDialogEntryKind::Unknown,
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {
                            WidgetryFileDialogEntryKind::DanglingSymlink
                        }
                        Err(error) => {
                            return Err(native_directory_io(
                                native_context(
                                    FileDialogDirectoryOperation::LinkMetadata,
                                    Some(directory.clone()),
                                    Some(path.clone()),
                                    Some(ordinal),
                                ),
                                error,
                            ));
                        }
                    }
                } else if file_type.is_dir() {
                    WidgetryFileDialogEntryKind::Directory
                } else if file_type.is_file() {
                    WidgetryFileDialogEntryKind::File
                } else {
                    WidgetryFileDialogEntryKind::Unknown
                };
                let (hidden, system) = (
                    Some(metadata.file_attributes() & 2 != 0),
                    Some(metadata.file_attributes() & 4 != 0),
                );
                let modified = metadata.modified().map_err(|error| {
                    native_directory_io(
                        native_context(
                            FileDialogDirectoryOperation::ModifiedTime,
                            Some(directory.clone()),
                            Some(path.clone()),
                            Some(ordinal),
                        ),
                        error,
                    )
                })?;
                Ok(WidgetryFileDialogEntryData {
                    path,
                    name,
                    kind,
                    size: Some(metadata.len()),
                    modified: Some(modified),
                    hidden,
                    system,
                })
            },
        )))
    }

    fn validate(&self, candidate: &WidgetryFileDialogCandidate) -> io::Result<bool> {
        match candidate.result() {
            WidgetryFileDialogResult::SavePath(path) => {
                let parent = path.parent().ok_or_else(|| {
                    native_request_io(
                        FileDialogRequestOperation::SaveParentMetadata,
                        Some(path.clone()),
                        None,
                        io::Error::other("save path has no parent"),
                    )
                })?;
                if !fs::metadata(parent)
                    .map_err(|error| {
                        native_request_io(
                            FileDialogRequestOperation::SaveParentMetadata,
                            Some(parent.to_owned()),
                            None,
                            error,
                        )
                    })?
                    .is_dir()
                {
                    return Err(native_request_io(
                        FileDialogRequestOperation::SaveParentMetadata,
                        Some(parent.to_owned()),
                        None,
                        io::Error::other("save parent is not a directory"),
                    ));
                }
                match fs::symlink_metadata(path) {
                    Ok(_) => {
                        if !fs::metadata(path)
                            .map_err(|error| {
                                native_request_io(
                                    FileDialogRequestOperation::SaveTargetMetadata,
                                    Some(path.clone()),
                                    None,
                                    error,
                                )
                            })?
                            .is_file()
                        {
                            return Err(native_request_io(
                                FileDialogRequestOperation::SaveTargetMetadata,
                                Some(path.clone()),
                                None,
                                io::Error::other("save target is not a file"),
                            ));
                        }
                        Ok(true)
                    }
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
                    Err(error) => Err(native_request_io(
                        FileDialogRequestOperation::SaveTargetLinkMetadata,
                        Some(path.clone()),
                        None,
                        error,
                    )),
                }
            }
            WidgetryFileDialogResult::File(path) => validate_path(path, false, None),
            WidgetryFileDialogResult::Directory(path) => validate_path(path, true, None),
            WidgetryFileDialogResult::Files(paths)
            | WidgetryFileDialogResult::Directories(paths) => {
                let directory =
                    matches!(candidate.result(), WidgetryFileDialogResult::Directories(_));
                for (index, path) in paths.iter().enumerate() {
                    validate_path(path, directory, Some(index))?;
                }
                Ok(true)
            }
            WidgetryFileDialogResult::Cancelled => {
                Err(io::Error::other("cancelled result cannot be validated"))
            }
        }
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        fs::create_dir(path).map_err(|error| {
            native_request_io(
                FileDialogRequestOperation::CreateDirectory,
                Some(path.to_owned()),
                None,
                error,
            )
        })
    }

    fn locations(&self) -> io::Result<Vec<WidgetryFileDialogLocation>> {
        let user = directories::UserDirs::new().ok_or_else(|| {
            native_request_io(
                FileDialogRequestOperation::UserDirectories,
                None,
                None,
                io::Error::other("user directories unavailable"),
            )
        })?;
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
        for letter in b'A'..=b'Z' {
            let path = PathBuf::from(format!("{}:\\", char::from(letter)));
            match fs::metadata(&path) {
                Ok(metadata) if metadata.is_dir() => locations.push(WidgetryFileDialogLocation {
                    name: format!("{}:", char::from(letter)),
                    path,
                }),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(native_request_io(
                        FileDialogRequestOperation::LocationMetadata,
                        Some(path),
                        None,
                        error,
                    ));
                }
            }
        }
        Ok(locations)
    }
}

fn validate_path(path: &Path, directory: bool, index: Option<usize>) -> io::Result<bool> {
    let metadata = fs::metadata(path).map_err(|error| {
        native_request_io(
            FileDialogRequestOperation::CandidateMetadata,
            Some(path.to_owned()),
            index,
            error,
        )
    })?;
    if (directory && metadata.is_dir()) || (!directory && metadata.is_file()) {
        Ok(true)
    } else {
        Err(native_request_io(
            FileDialogRequestOperation::CandidateMetadata,
            Some(path.to_owned()),
            index,
            io::Error::other("candidate kind changed"),
        ))
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_missing_save_target_is_an_expected_candidate_without_diagnostics() {
        let directory = tempfile::tempdir().unwrap();
        let candidate = crate::WidgetryFileDialogCandidate {
            token: crate::WidgetryFileDialogState::new(Default::default())
                .unwrap()
                .token(),
            result: WidgetryFileDialogResult::SavePath(directory.path().join("new.txt")),
        };
        let logs = bevy_widgetry_test_utils::LogCapture::default();
        assert!(
            !logs
                .run(|| WidgetryFileDialogNativeFileSystem.validate(&candidate))
                .unwrap()
        );
        assert!(logs.records().is_empty());
    }

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
