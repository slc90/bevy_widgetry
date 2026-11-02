use crate::error::native_preference_io;
use crate::{FileDialogPreferenceFault as Fault, FileDialogPreferenceOperation as Phase};
use crate::{
    WidgetryFileDialogFilterId, WidgetryFileDialogSort, WidgetryFileDialogStorageSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read, Write};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

const MAX_BYTES: usize = 1_048_576;

#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    scopes: BTreeMap<String, Preferences>,
}

#[derive(Serialize, Deserialize)]
struct Preferences {
    visited: Option<EncodedPath>,
    picked: Option<EncodedPath>,
    hidden: bool,
    system: bool,
    filter: String,
    sort: u8,
    pinned: Vec<EncodedPath>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "platform", content = "units")]
enum EncodedPath {
    Windows(Vec<u16>),
}

#[derive(Debug)]
enum PreferenceCause {
    Format(Fault),
    Json(serde_json::Error),
}
impl std::fmt::Display for PreferenceCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Format(error) => std::fmt::Display::fmt(error, f),
            Self::Json(error) => std::fmt::Display::fmt(error, f),
        }
    }
}
impl std::error::Error for PreferenceCause {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Format(error) => error,
            Self::Json(error) => error,
        })
    }
}
fn invalid(fault: Fault) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, PreferenceCause::Format(fault))
}
fn json_error(error: serde_json::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, PreferenceCause::Json(error))
}
fn at(phase: Phase, scope: Option<&str>, error: io::Error) -> io::Error {
    native_preference_io(phase, scope, None, error)
}

fn encode(path: &Path) -> io::Result<EncodedPath> {
    if !path.is_absolute() || path.as_os_str().len() > 32768 {
        return Err(invalid(Fault::Path {
            path: path.to_owned(),
            issue: "relative or oversized",
        }));
    }
    let units: Vec<_> = path.as_os_str().encode_wide().collect();
    if units.contains(&0) {
        return Err(invalid(Fault::Path {
            path: path.to_owned(),
            issue: "NUL",
        }));
    }
    Ok(EncodedPath::Windows(units))
}

fn decode(encoded: EncodedPath) -> io::Result<PathBuf> {
    let EncodedPath::Windows(units) = encoded;
    if units.len() > 32768 || units.contains(&0) {
        return Err(invalid(Fault::Path {
            path: PathBuf::from(OsString::from_wide(&units)),
            issue: "oversized or NUL encoded path",
        }));
    }
    let path = PathBuf::from(OsString::from_wide(&units));
    if !path.is_absolute() {
        return Err(invalid(Fault::Path {
            path,
            issue: "relative",
        }));
    }
    Ok(path)
}

pub(crate) fn load(path: &Path) -> io::Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(at(Phase::Open, None, error)),
    };
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| at(Phase::Read, None, error))?;
    if bytes.len() > MAX_BYTES {
        return Err(at(
            Phase::Read,
            None,
            invalid(Fault::FileCapacity {
                actual: bytes.len(),
                limit: MAX_BYTES,
            }),
        ));
    }
    let document: Document = serde_json::from_slice(&bytes)
        .map_err(|error| at(Phase::DecodeJson, None, json_error(error)))?;
    if document.version != 1 || document.scopes.len() > 256 {
        return Err(at(
            Phase::DecodeValues,
            None,
            invalid(Fault::Schema {
                version: document.version,
                scopes: document.scopes.len(),
            }),
        ));
    }
    let mut result = BTreeMap::new();
    for (scope, prefs) in document.scopes {
        if scope.is_empty()
            || scope.len() > 128
            || prefs.pinned.len() > 1024
            || prefs.filter.len() > 1024
        {
            return Err(at(
                Phase::DecodeValues,
                Some(&scope),
                invalid(Fault::ScopeValues {
                    scope: scope.clone(),
                    bytes: scope.len(),
                    pinned: prefs.pinned.len(),
                    filter_bytes: prefs.filter.len(),
                }),
            ));
        }
        let sort = match prefs.sort {
            0 => WidgetryFileDialogSort::NameAscending,
            1 => WidgetryFileDialogSort::NameDescending,
            2 => WidgetryFileDialogSort::SizeAscending,
            3 => WidgetryFileDialogSort::ModifiedDescending,
            _ => {
                return Err(at(
                    Phase::DecodeValues,
                    Some(&scope),
                    invalid(Fault::Sort { value: prefs.sort }),
                ));
            }
        };
        let pinned = prefs
            .pinned
            .into_iter()
            .map(decode)
            .collect::<io::Result<Vec<_>>>()
            .map_err(|error| at(Phase::DecodeValues, Some(&scope), error))?;
        let visited = prefs
            .visited
            .map(decode)
            .transpose()
            .map_err(|error| at(Phase::DecodeValues, Some(&scope), error))?;
        let picked = prefs
            .picked
            .map(decode)
            .transpose()
            .map_err(|error| at(Phase::DecodeValues, Some(&scope), error))?;
        let mut unique = std::collections::BTreeSet::new();
        if let Some(path) = pinned.iter().find(|path| !unique.insert(*path)) {
            return Err(at(
                Phase::DecodeValues,
                Some(&scope),
                invalid(Fault::DuplicatePinned { path: path.clone() }),
            ));
        }
        result.insert(
            scope,
            WidgetryFileDialogStorageSnapshot {
                last_visited_dir: visited,
                last_picked_dir: picked,
                show_hidden: prefs.hidden,
                show_system: prefs.system,
                filter: WidgetryFileDialogFilterId(prefs.filter),
                sort,
                pinned,
            },
        );
    }
    Ok(result)
}

pub(crate) fn save(
    path: &Path,
    scopes: &BTreeMap<String, WidgetryFileDialogStorageSnapshot>,
) -> io::Result<()> {
    if scopes.len() > 256 {
        return Err(at(
            Phase::EncodeValues,
            None,
            invalid(Fault::ScopeCapacity {
                actual: scopes.len(),
                limit: 256,
            }),
        ));
    }
    let mut encoded = BTreeMap::new();
    for (scope, prefs) in scopes {
        if scope.is_empty()
            || scope.len() > 128
            || prefs.pinned.len() > 1024
            || prefs.filter.0.len() > 1024
        {
            return Err(at(
                Phase::EncodeValues,
                Some(scope),
                invalid(Fault::ScopeValues {
                    scope: scope.clone(),
                    bytes: scope.len(),
                    pinned: prefs.pinned.len(),
                    filter_bytes: prefs.filter.0.len(),
                }),
            ));
        }
        encoded.insert(
            scope.clone(),
            Preferences {
                visited: prefs
                    .last_visited_dir
                    .as_deref()
                    .map(encode)
                    .transpose()
                    .map_err(|error| at(Phase::EncodeValues, Some(scope), error))?,
                picked: prefs
                    .last_picked_dir
                    .as_deref()
                    .map(encode)
                    .transpose()
                    .map_err(|error| at(Phase::EncodeValues, Some(scope), error))?,
                hidden: prefs.show_hidden,
                system: prefs.show_system,
                filter: prefs.filter.0.clone(),
                sort: match prefs.sort {
                    WidgetryFileDialogSort::NameAscending => 0,
                    WidgetryFileDialogSort::NameDescending => 1,
                    WidgetryFileDialogSort::SizeAscending => 2,
                    WidgetryFileDialogSort::ModifiedDescending => 3,
                },
                pinned: prefs
                    .pinned
                    .iter()
                    .map(|path| encode(path))
                    .collect::<io::Result<Vec<_>>>()
                    .map_err(|error| at(Phase::EncodeValues, Some(scope), error))?,
            },
        );
    }
    let bytes = serde_json::to_vec(&Document {
        version: 1,
        scopes: encoded,
    })
    .map_err(|error| at(Phase::EncodeJson, None, json_error(error)))?;
    if bytes.len() > MAX_BYTES {
        return Err(at(
            Phase::EncodeJson,
            None,
            invalid(Fault::FileCapacity {
                actual: bytes.len(),
                limit: MAX_BYTES,
            }),
        ));
    }
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
        native_preference_io(Phase::CreateTemporary, None, Some(parent.to_owned()), error)
    })?;
    temporary.write_all(&bytes).map_err(|error| {
        native_preference_io(
            Phase::WriteTemporary,
            None,
            Some(temporary.path().to_owned()),
            error,
        )
    })?;
    temporary.as_file().sync_all().map_err(|error| {
        native_preference_io(
            Phase::SyncTemporary,
            None,
            Some(temporary.path().to_owned()),
            error,
        )
    })?;
    temporary.persist(path).map_err(|error| {
        native_preference_io(Phase::Replace, None, Some(path.to_owned()), error.error)
    })?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn malformed_json_preserves_the_original_parser_cause() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.json");
        fs::write(&path, b"{broken").unwrap();
        let error = load(&path).unwrap_err();
        let mut cause: Option<&(dyn std::error::Error + 'static)> = Some(&error);
        let mut parser = None;
        while let Some(error) = cause {
            if let Some(error) = error.downcast_ref::<serde_json::Error>() {
                parser = Some(error);
                break;
            }
            cause = error.source();
        }
        let parser = parser.expect("original serde_json::Error missing");
        assert_eq!(parser.classify(), serde_json::error::Category::Syntax);
        assert_eq!(parser.line(), 1);
    }

    #[test]
    fn preferences_round_trip_and_replace_existing_file() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("preferences.json");
        let mut scopes =
            BTreeMap::from([("open".into(), WidgetryFileDialogStorageSnapshot::default())]);
        save(&path, &scopes)?;
        scopes
            .get_mut("open")
            .ok_or_else(|| io::Error::other("scope missing"))?
            .show_hidden = true;
        save(&path, &scopes)?;
        assert_eq!(load(&path)?, scopes);
        fs::write(&path, b"{\"version\":999,\"scopes\":{}}")?;
        assert!(load(&path).is_err());
        fs::write(&path, b"broken")?;
        assert!(load(&path).is_err());
        Ok(())
    }

    #[test]
    fn existing_windows_v1_document_keeps_its_format_and_paths() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("preferences.json");
        let document = serde_json::json!({
            "version": 1,
            "scopes": {
                "open": {
                    "visited": {"platform": "Windows", "units": [67, 58, 92, 0xd800]},
                    "picked": {"platform": "Windows", "units": [67, 58, 92, 112]},
                    "hidden": true,
                    "system": false,
                    "filter": "all",
                    "sort": 0,
                    "pinned": [{"platform": "Windows", "units": [67, 58, 92, 113]}]
                }
            }
        });
        fs::write(&path, serde_json::to_vec(&document)?)?;
        let expected = BTreeMap::from([(
            "open".into(),
            WidgetryFileDialogStorageSnapshot {
                last_visited_dir: Some(OsString::from_wide(&[67, 58, 92, 0xd800]).into()),
                last_picked_dir: Some(PathBuf::from("C:\\p")),
                show_hidden: true,
                filter: WidgetryFileDialogFilterId("all".into()),
                pinned: vec![PathBuf::from("C:\\q")],
                ..Default::default()
            },
        )]);
        let scopes = load(&path)?;
        assert_eq!(scopes, expected);
        save(&path, &scopes)?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path)?)?,
            document
        );
        Ok(())
    }

    #[test]
    fn unsupported_path_encoding_rejects_preferences() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("preferences.json");
        let document = serde_json::json!({
            "version": 1,
            "scopes": {
                "open": {
                    "visited": {"platform": "Unix", "units": [47, 116, 109, 112]},
                    "picked": null,
                    "hidden": true,
                    "system": false,
                    "filter": "all",
                    "sort": 0,
                    "pinned": []
                }
            }
        });
        fs::write(&path, serde_json::to_vec(&document)?)?;
        let error = load(&path).expect_err("unsupported path encoding must fail");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path)?)?,
            document
        );
        Ok(())
    }

    #[test]
    fn windows_unpaired_utf16_round_trips_without_loss() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("preferences.json");
        let raw = std::ffi::OsString::from_wide(&[67, 58, 92, 0xd800]);
        let scopes = BTreeMap::from([(
            "open".into(),
            WidgetryFileDialogStorageSnapshot {
                last_visited_dir: Some(raw.into()),
                ..Default::default()
            },
        )]);
        save(&path, &scopes)?;
        assert_eq!(load(&path)?, scopes);
        Ok(())
    }

    #[test]
    fn invalid_nul_path_cannot_replace_valid_preferences() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("preferences.json");
        let mut scopes =
            BTreeMap::from([("open".into(), WidgetryFileDialogStorageSnapshot::default())]);
        save(&path, &scopes)?;
        let original = fs::read(&path)?;
        scopes
            .get_mut("open")
            .ok_or_else(|| io::Error::other("scope missing"))?
            .last_visited_dir = Some(temporary.path().join("bad\0name"));
        assert!(save(&path, &scopes).is_err());
        assert_eq!(fs::read(&path)?, original);
        assert!(load(&path)?.contains_key("open"));
        Ok(())
    }

    #[test]
    fn failed_replace_preserves_the_existing_file() -> io::Result<()> {
        let temporary = tempfile::tempdir()?;
        let path = temporary.path().join("preferences.json");
        let scopes =
            BTreeMap::from([("open".into(), WidgetryFileDialogStorageSnapshot::default())]);
        save(&path, &scopes)?;
        let original = fs::read(&path)?;
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)?;
        assert!(save(&path, &scopes).is_err());
        drop(locked);
        assert_eq!(fs::read(&path)?, original);
        Ok(())
    }
}
