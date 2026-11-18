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
        if !matches!(
            self.mode,
            WidgetryFileDialogMode::PickFile | WidgetryFileDialogMode::SaveFile
        ) {
            return Err(self.contract(FileDialogStateFault::FilenameMode));
        }
        if self.mode == WidgetryFileDialogMode::SaveFile && self.filename() == filename {
            return Ok(false);
        }
        self.update_filename(Some(filename))
    }

    pub(crate) fn clear_pick_filename(&mut self) -> Result<bool, FileDialogFailure> {
        if self.mode != WidgetryFileDialogMode::PickFile {
            return Ok(false);
        }
        self.update_filename(None)
    }

    fn update_filename(&mut self, filename: Option<OsString>) -> Result<bool, FileDialogFailure> {
        if self.filename == filename {
            return Ok(false);
        }
        self.advance_edit_revision()?;
        self.filename = filename;
        self.filename_rejection = None;
        self.confirmation = WidgetryFileDialogConfirmation::Idle;
        self.candidate = None;
        self.validation_job = None;
        self.activate_after_selection = false;
        Ok(true)
    }

    pub fn can_confirm(&self) -> bool {
        self.confirmation_ready()
            && match self.mode {
                WidgetryFileDialogMode::PickFile => self
                    .filename
                    .as_ref()
                    .map_or_else(|| !self.selected.is_empty(), |name| !name.is_empty()),
                WidgetryFileDialogMode::SaveFile => !self.filename().is_empty(),
                WidgetryFileDialogMode::PickDirectory => true,
                _ => !self.selected.is_empty(),
            }
    }

    pub(crate) fn confirmation_ready(&self) -> bool {
        self.session_state == WidgetryFileDialogSessionState::Open
            && self.confirmation == WidgetryFileDialogConfirmation::Idle
            && !self.selection_pending()
            && !self.projection_pending
            && self.confirmation_directory().is_ok()
    }

    fn confirmation_directory(&self) -> Result<&Path, FileDialogStateFault> {
        if self.started_generation != Some(self.token.generation)
            || matches!(
                self.directory_state,
                WidgetryFileDialogDirectoryState::Failed(_)
            )
        {
            return Err(FileDialogStateFault::DirectoryUnavailable);
        }
        self.current_path()
            .ok_or(FileDialogStateFault::CurrentDirectoryMissing)
    }

    fn pick_filename_entry(
        &self,
        directory: &Path,
        name: &OsStr,
    ) -> Result<WidgetryFileDialogEntryId, &'static str> {
        const INVALID: &str = "File name: Enter a single file name from this folder.";
        const MISSING: &str = "File name: No matching file in this folder or file type.";
        const DIRECTORY: &str = "File name: Choose a file from this folder.";

        let snapshot = self.snapshot.as_ref().ok_or(MISSING)?;
        // A selected row carries its original UTF-16 identity even when its editor displays replacement characters.
        if let Some(entry) = self.selected.first().and_then(|id| snapshot.entry(*id))
            && snapshot.positions.contains_key(&entry.id)
            && entry.kind.is_file()
            && (entry.name == name
                || name
                    .to_str()
                    .is_some_and(|text| entry.name.to_string_lossy() == text))
        {
            return Ok(entry.id);
        }
        valid_name(name).map_err(|_| INVALID)?;
        if let Some(entry) = snapshot
            .paths
            .get(&directory.join(name))
            .and_then(|id| snapshot.entry(*id))
            && entry.name == name
        {
            if !snapshot.positions.contains_key(&entry.id) {
                return Err(MISSING);
            }
            return if entry.kind.is_file() {
                Ok(entry.id)
            } else {
                Err(DIRECTORY)
            };
        }
        let text = name.to_str().ok_or(MISSING)?;
        let mut matches = snapshot.entries().iter().filter(|entry| {
            snapshot.positions.contains_key(&entry.id)
                && entry
                    .name
                    .to_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case(text))
        });
        let entry = matches.next().ok_or(MISSING)?;
        if matches.next().is_some() {
            return Err(MISSING);
        }
        if entry.kind.is_file() {
            Ok(entry.id)
        } else {
            Err(DIRECTORY)
        }
    }

    pub(crate) fn confirm(&mut self) -> Result<bool, FileDialogFailure> {
        if self.confirmation != WidgetryFileDialogConfirmation::Idle {
            return Ok(false);
        }
        if self.selection_pending() || self.projection_pending {
            return Err(self.contract(FileDialogStateFault::ConfirmationPending));
        }
        let directory = self
            .confirmation_directory()
            .map_err(|fault| self.contract(fault))?;
        if self.mode == WidgetryFileDialogMode::PickFile
            && let Some(name) = &self.filename
        {
            match self.pick_filename_entry(directory, name) {
                Ok(id) if !self.selected.contains(&id) => {
                    self.select(WidgetryFileDialogAction::Select {
                        id,
                        operation: WidgetryFileDialogSelection::Replace,
                        token: self.token,
                    })?;
                    self.activate_after_selection = true;
                    self.error = None;
                    return Ok(true);
                }
                Ok(_) => {}
                Err(message) => {
                    if self.filename_rejection == Some(message) {
                        return Ok(false);
                    }
                    self.filename_rejection = Some(message);
                    return Ok(true);
                }
            }
        }
        if self.selected.is_empty()
            && !matches!(
                self.mode,
                WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::SaveFile
            )
        {
            return Err(self.contract(FileDialogStateFault::ConfirmationSelectionMissing));
        }
        let directory = directory.to_owned();
        self.advance_edit_revision()?;
        self.validation_job = Some(WidgetryFileDialogValidationJob {
            token: self.token,
            mode: self.mode,
            directory,
            snapshot: self.snapshot.clone(),
            selected: self.selected.clone(),
            filename: self.filename.clone().unwrap_or_default(),
            filter: self.query().filter,
            allow_different_extension: self.allow_different_extension,
        });
        self.confirmation = WidgetryFileDialogConfirmation::Validating;
        self.filename_rejection = None;
        self.error = None;
        Ok(true)
    }

    fn advance_edit_revision(&mut self) -> Result<(), FileDialogFailure> {
        self.token.edit_revision = self.token.edit_revision.checked_add(1).ok_or_else(|| {
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
        Ok(())
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
