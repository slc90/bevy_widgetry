use crate::model::contract_error;
use crate::*;
use bevy::prelude::BevyError;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
#[cfg(windows)]
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
    #[cfg(windows)]
    {
        let units: Vec<_> = name.encode_wide().collect();
        if units.iter().any(|unit| {
            *unit < 32
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
    }
    #[cfg(unix)]
    {
        if name.as_encoded_bytes().contains(&0) {
            return Err("filename contains NUL".into());
        }
    }
    Ok(())
}

impl WidgetryFileDialogValidationJob {
    pub fn token(&self) -> WidgetryFileDialogToken {
        self.token
    }

    pub fn prepare(self) -> Result<WidgetryFileDialogCandidate, String> {
        let result = if self.mode == WidgetryFileDialogMode::SaveFile {
            valid_name(&self.filename)?;
            let mut filename = self.filename;
            if Path::new(&filename).extension().is_none() {
                if let Some(extension) = &self.filter.default_extension {
                    filename.push(".");
                    filename.push(extension);
                }
            } else if !self.allow_different_extension && !self.filter.matches(&filename) {
                return Err("filename suffix does not match filter".into());
            }
            valid_name(&filename)?;
            WidgetryFileDialogResult::SavePath(self.directory.join(filename))
        } else if self.mode == WidgetryFileDialogMode::PickDirectory && self.selected.is_empty() {
            WidgetryFileDialogResult::Directory(self.directory)
        } else {
            let snapshot = self.snapshot.ok_or("selection has no directory snapshot")?;
            let paths: Arc<[PathBuf]> = snapshot
                .visible
                .iter()
                .filter(|id| self.selected.contains(id))
                .map(|id| {
                    snapshot
                        .entry(*id)
                        .map(|entry| entry.path.clone())
                        .ok_or_else(|| "selected EntryId is missing".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?
                .into();
            if paths.len() != self.selected.len() {
                return Err("selection is outside visible snapshot".into());
            }
            for id in self.selected.iter() {
                if !snapshot
                    .entry(*id)
                    .is_some_and(|entry| self.mode.accepts(entry.kind))
                {
                    return Err("selection kind incompatible with mode".into());
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
                _ => return Err("selection count incompatible with mode".into()),
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
    pub(crate) fn set_filename(&mut self, filename: OsString) -> Result<bool, BevyError> {
        if self.mode != WidgetryFileDialogMode::SaveFile {
            return Err(contract_error("filename requires SaveFile mode"));
        }
        if self.filename == filename {
            return Ok(false);
        }
        let revision = self
            .token
            .edit_revision
            .checked_add(1)
            .ok_or_else(|| contract_error("edit revision exhausted"))?;
        self.filename = filename;
        self.token.edit_revision = revision;
        self.confirmation = WidgetryFileDialogConfirmation::Idle;
        self.candidate = None;
        self.validation_job = None;
        Ok(true)
    }

    pub(crate) fn confirm(&mut self) -> Result<bool, BevyError> {
        if self.confirmation != WidgetryFileDialogConfirmation::Idle {
            return Ok(false);
        }
        if self.selection_pending() || self.projection_pending {
            return Err(contract_error("selection or projection pending"));
        }
        if self.started_generation != Some(self.token.generation)
            || matches!(
                self.directory_state,
                WidgetryFileDialogDirectoryState::Failed(_)
            )
        {
            return Err(contract_error(
                "directory is not available for confirmation",
            ));
        }
        let directory = self
            .current_path
            .clone()
            .ok_or_else(|| contract_error("current directory missing"))?;
        if self.selected.is_empty()
            && !matches!(
                self.mode,
                WidgetryFileDialogMode::PickDirectory | WidgetryFileDialogMode::SaveFile
            )
        {
            return Err(contract_error("confirmation requires selection"));
        }
        let revision = self
            .token
            .edit_revision
            .checked_add(1)
            .ok_or_else(|| contract_error("edit revision exhausted"))?;
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
    ) -> Result<bool, BevyError> {
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
    ) -> Result<bool, BevyError> {
        if !matches!(self.confirmation, WidgetryFileDialogConfirmation::AwaitingOverwrite { token: pending, .. } if pending == token)
            || self.token != token
        {
            return Err(contract_error("stale overwrite decision"));
        }
        if !accept {
            self.confirmation = WidgetryFileDialogConfirmation::Idle;
            self.candidate = None;
            return Ok(true);
        }
        let result = self
            .candidate
            .take()
            .ok_or_else(|| contract_error("overwrite candidate missing"))?;
        Ok(self.resolve(result))
    }
}

#[cfg(test)]
// 测试允许断言，workspace 的 macro 禁令只约束生产代码。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! filename 的纯语法校验不访问 filesystem。
    //! path component 与 Windows reserved name 是 guard，原始 OsString 必须保持。
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

    #[cfg(windows)]
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
