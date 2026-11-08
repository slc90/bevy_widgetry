const FIRST_PRINTABLE_UTF16_UNIT: u16 = 32;

use crate::*;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::OsStrExt;
use std::path::{Component as PathComponent, Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogValidationJob {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) mode: WidgetryFileDialogMode,
    pub(crate) directory: PathBuf,
    pub(crate) snapshot: Option<Arc<WidgetryFileDialogSnapshot>>,
    pub(crate) selected: Arc<BTreeSet<WidgetryFileDialogEntryId>>,
    pub(crate) filename: OsString,
    pub(crate) filter: WidgetryFileDialogFilter,
    pub(crate) allow_different_extension: bool,
}

#[derive(Clone, Debug)]
pub struct WidgetryFileDialogCandidate {
    pub(crate) token: WidgetryFileDialogToken,
    pub(crate) result: WidgetryFileDialogResult,
}

pub(crate) fn valid_name(name: &OsStr) -> Result<(), String> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(PathComponent::Normal(_))) || components.next().is_some() {
        return Err("name must be one ordinary path component".into());
    }
    let units: Vec<_> = name.encode_wide().collect();
    if units.iter().any(|unit| {
        *unit < FIRST_PRINTABLE_UTF16_UNIT
            || b"<>:\"/\\|?*"
                .iter()
                .any(|character| *unit == u16::from(*character))
    }) || units
        .last()
        .is_some_and(|unit| *unit == u16::from(b'.') || *unit == u16::from(b' '))
    {
        return Err("invalid Windows filename characters".into());
    }
    let stem: Vec<_> = units
        .iter()
        .copied()
        .take_while(|unit| *unit != u16::from(b'.'))
        .collect();
    let stem = String::from_utf16_lossy(&stem)
        .trim_end()
        .to_ascii_uppercase();
    if ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|tail| {
                matches!(
                    tail,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        })
    {
        return Err("reserved Windows filename".into());
    }
    Ok(())
}

impl WidgetryFileDialogValidationJob {
    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn prepare(self) -> Result<WidgetryFileDialogCandidate, FileDialogRequestError> {
        let context = FileDialogRequestContext {
            root: None,
            token: self.token,
            mode: Some(self.mode),
            operation: FileDialogRequestOperation::PrepareCandidate,
            path: Some(self.directory.clone()),
            resource: None,
            paths: None,
            index: None,
        };
        let fault = |fault| {
            FileDialogRequestError::Failed(
                crate::error::request_failure(context.clone(), fault).into(),
            )
        };
        let result = if self.mode == WidgetryFileDialogMode::SaveFile {
            valid_name(&self.filename).map_err(FileDialogRequestError::Rejected)?;
            let mut filename = self.filename;
            if Path::new(&filename).extension().is_none() {
                if let Some(extension) = &self.filter.default_extension {
                    filename.push(".");
                    filename.push(extension);
                }
            } else if !self.allow_different_extension && !self.filter.matches(&filename) {
                return Err(FileDialogRequestError::Rejected(
                    "filename suffix does not match filter".into(),
                ));
            }
            valid_name(&filename).map_err(FileDialogRequestError::Rejected)?;
            WidgetryFileDialogResult::SavePath(self.directory.join(filename))
        } else if self.mode == WidgetryFileDialogMode::PickDirectory && self.selected.is_empty() {
            WidgetryFileDialogResult::Directory(self.directory)
        } else {
            let snapshot = self
                .snapshot
                .ok_or_else(|| fault(FileDialogRequestFault::SelectionSnapshotMissing))?;
            let paths: Arc<[PathBuf]> = snapshot
                .visible
                .iter()
                .filter(|id| self.selected.contains(id))
                .map(|id| {
                    snapshot
                        .entry(*id)
                        .map(|entry| entry.path.clone())
                        .ok_or_else(|| {
                            fault(FileDialogRequestFault::SelectedEntryMissing { id: *id })
                        })
                })
                .collect::<Result<Vec<_>, _>>()?
                .into();
            if paths.len() != self.selected.len() {
                return Err(fault(FileDialogRequestFault::SelectionOutsideSnapshot {
                    selected: self.selected.len(),
                    visible: paths.len(),
                }));
            }
            for id in self.selected.iter() {
                let entry = snapshot.entry(*id).ok_or_else(|| {
                    fault(FileDialogRequestFault::SelectedEntryMissing { id: *id })
                })?;
                if !self.mode.accepts(entry.kind) {
                    return Err(fault(FileDialogRequestFault::SelectionKind {
                        id: *id,
                        kind: entry.kind,
                    }));
                }
            }
            match self.mode {
                WidgetryFileDialogMode::PickFile if paths.len() == 1 => {
                    WidgetryFileDialogResult::File(paths[0].clone())
                }
                WidgetryFileDialogMode::PickDirectory if paths.len() == 1 => {
                    WidgetryFileDialogResult::Directory(paths[0].clone())
                }
                WidgetryFileDialogMode::PickFiles if !paths.is_empty() => {
                    WidgetryFileDialogResult::Files(paths)
                }
                WidgetryFileDialogMode::PickDirectories if !paths.is_empty() => {
                    WidgetryFileDialogResult::Directories(paths)
                }
                _ => {
                    return Err(fault(FileDialogRequestFault::SelectionCount {
                        actual: paths.len(),
                    }));
                }
            }
        };
        Ok(WidgetryFileDialogCandidate {
            token: self.token,
            result,
        })
    }
}

impl WidgetryFileDialogCandidate {
    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn result(&self) -> &WidgetryFileDialogResult {
        &self.result
    }
}

impl WidgetryFileDialogState {
    pub(crate) fn set_filename(&mut self, filename: OsString) -> Result<bool, FileDialogFailure> {
        if self.mode != WidgetryFileDialogMode::SaveFile {
            return Err(self.contract(FileDialogStateFault::FilenameMode));
        }
        if self.filename == filename {
            return Ok(false);
        }
        let revision = self.token.edit_revision.checked_add(1).ok_or_else(|| {
            crate::error::failure(
                FileDialogError::RevisionExhausted {
                    counter: "edit_revision",
                    value: self.token.edit_revision,
                    token: Some(Box::new(self.token)),
                    scope: None,
                },
                "file_dialog.confirmation",
            )
        })?;
        self.filename = filename;
        self.token.edit_revision = revision;
        self.confirmation = WidgetryFileDialogConfirmation::Idle;
        self.candidate = None;
        self.validation_job = None;
        Ok(true)
    }

    pub(crate) fn confirm(&mut self) -> Result<bool, FileDialogFailure> {
        if self.confirmation != WidgetryFileDialogConfirmation::Idle {
            return Ok(false);
        }
        if self.selection_pending() || self.projection_pending {
            return Err(self.contract(FileDialogStateFault::ConfirmationPending));
        }
        if self.started_generation != Some(self.token.generation)
            || matches!(
                self.directory_state,
                WidgetryFileDialogDirectoryState::Failed(_)
            )
        {
            return Err(self.contract(FileDialogStateFault::DirectoryUnavailable));
        }
        let directory = self
            .current_path
            .clone()
            .ok_or_else(|| self.contract(FileDialogStateFault::CurrentDirectoryMissing))?;
        if self.selected.is_empty()
            && !matches!(
                self.mode,
                WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::SaveFile
            )
        {
            return Err(self.contract(FileDialogStateFault::ConfirmationSelectionMissing));
        }
        let revision = self.token.edit_revision.checked_add(1).ok_or_else(|| {
            crate::error::failure(
                FileDialogError::RevisionExhausted {
                    counter: "edit_revision",
                    value: self.token.edit_revision,
                    token: Some(Box::new(self.token)),
                    scope: None,
                },
                "file_dialog.confirmation",
            )
        })?;
        self.token.edit_revision = revision;
        self.validation_job = Some(WidgetryFileDialogValidationJob {
            token: self.token,
            mode: self.mode,
            directory,
            snapshot: self.snapshot.clone(),
            selected: self.selected.clone(),
            filename: self.filename.clone(),
            filter: self.query().filter,
            allow_different_extension: self.allow_different_extension,
        });
        self.confirmation = WidgetryFileDialogConfirmation::Validating;
        self.error = None;
        Ok(true)
    }

    pub(crate) fn validated(
        &mut self,
        candidate: WidgetryFileDialogCandidate,
        exists: bool,
    ) -> Result<bool, FileDialogFailure> {
        if candidate.token != self.token
            || self.confirmation != WidgetryFileDialogConfirmation::Validating
        {
            return Ok(false);
        }
        if let WidgetryFileDialogResult::SavePath(path) = &candidate.result
            && exists
        {
            self.confirmation = WidgetryFileDialogConfirmation::AwaitingOverwrite {
                path: path.clone(),
                token: candidate.token,
            };
            self.candidate = Some(candidate.result);
            self.validation_job = None;
            return Ok(true);
        }
        Ok(self.resolve(candidate.result))
    }

    pub(crate) fn overwrite(
        &mut self,
        token: WidgetryFileDialogToken,
        accept: bool,
    ) -> Result<bool, FileDialogFailure> {
        if !matches!(self.confirmation, WidgetryFileDialogConfirmation::AwaitingOverwrite { token: pending, .. } if pending == token)
            || self.token != token
        {
            return Err(self.contract(FileDialogStateFault::StaleOverwrite { requested: token }));
        }
        if !accept {
            self.confirmation = WidgetryFileDialogConfirmation::Idle;
            self.candidate = None;
            return Ok(true);
        }
        let result = self
            .candidate
            .take()
            .ok_or_else(|| self.contract(FileDialogStateFault::MissingOverwriteCandidate))?;
        Ok(self.resolve(result))
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn filename_must_be_one_component() {
        for name in ["", ".", "..", "one/two"] {
            assert!(valid_name(OsStr::new(name)).is_err());
        }
        for name in ["report.txt", "文件", ".config", "archive.tar.gz"] {
            assert!(valid_name(OsStr::new(name)).is_ok());
        }
    }

    #[test]
    fn windows_reserved_names_and_trailing_characters_are_rejected() {
        for name in [
            "CON", "con.txt", "nul", "LPT9.txt", "COM¹", "bad.", "bad ", "a:b", "a?b", "a\u{0}b",
        ] {
            assert!(valid_name(OsStr::new(name)).is_err(), "{name:?}");
        }
        assert!(valid_name(OsStr::new("COM10.txt")).is_ok());
    }
}
