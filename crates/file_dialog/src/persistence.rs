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

fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

fn encode(path: &Path) -> io::Result<EncodedPath> {
    if !path.is_absolute() || path.as_os_str().len() > 32768 {
        return Err(invalid("invalid or oversized stored path"));
    }
    let units: Vec<_> = path.as_os_str().encode_wide().collect();
    if units.contains(&0) {
        return Err(invalid("stored path contains NUL"));
    }
    Ok(EncodedPath::Windows(units))
}

fn decode(encoded: EncodedPath) -> io::Result<PathBuf> {
    let EncodedPath::Windows(units) = encoded;
    if units.len() > 32768 || units.contains(&0) {
        return Err(invalid("invalid encoded Windows path"));
    }
    let path = PathBuf::from(OsString::from_wide(&units));
    if !path.is_absolute() {
        return Err(invalid("stored path must be absolute"));
    }
    Ok(path)
}

pub(crate) fn load(path: &Path) -> io::Result<BTreeMap<String, WidgetryFileDialogStorageSnapshot>> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(error),
    };
    let mut bytes = Vec::new();
    file.take((MAX_BYTES + 1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid("preference file capacity exceeded"));
    }
    let document: Document = serde_json::from_slice(&bytes).map_err(invalid)?;
    if document.version != 1 || document.scopes.len() > 256 {
        return Err(invalid("unsupported schema or scope capacity"));
    }
    let mut result = BTreeMap::new();
    for (scope, prefs) in document.scopes {
        if scope.is_empty()
            || scope.len() > 128
            || prefs.pinned.len() > 1024
            || prefs.filter.len() > 1024
        {
            return Err(invalid("preference capacity exceeded"));
        }
        let sort = match prefs.sort {
            0 => WidgetryFileDialogSort::NameAscending,
            1 => WidgetryFileDialogSort::NameDescending,
            2 => WidgetryFileDialogSort::SizeAscending,
            3 => WidgetryFileDialogSort::ModifiedDescending,
            _ => return Err(invalid("unknown sorting preference")),
        };
        let pinned = prefs
            .pinned
            .into_iter()
            .map(decode)
            .collect::<io::Result<Vec<_>>>()?;
        let visited = prefs.visited.map(decode).transpose()?;
        let picked = prefs.picked.map(decode).transpose()?;
        let mut unique = std::collections::BTreeSet::new();
        if pinned.iter().any(|path| !unique.insert(path)) {
            return Err(invalid("duplicate pinned path"));
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
        return Err(invalid("scope capacity exceeded"));
    }
    let mut encoded = BTreeMap::new();
    for (scope, prefs) in scopes {
        if scope.is_empty()
            || scope.len() > 128
            || prefs.pinned.len() > 1024
            || prefs.filter.0.len() > 1024
        {
            return Err(invalid("preference capacity exceeded"));
        }
        encoded.insert(
            scope.clone(),
            Preferences {
                visited: prefs.last_visited_dir.as_deref().map(encode).transpose()?,
                picked: prefs.last_picked_dir.as_deref().map(encode).transpose()?,
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
                    .collect::<io::Result<Vec<_>>>()?,
            },
        );
    }
    let bytes = serde_json::to_vec(&Document {
        version: 1,
        scopes: encoded,
    })
    .map_err(invalid)?;
    if bytes.len() > MAX_BYTES {
        return Err(invalid("preference file capacity exceeded"));
    }
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&bytes)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
// 测试断言用于保护 persistence contract，不适用生产 macro 禁令。
#[allow(clippy::disallowed_macros)]
mod tests {
    //! persistence state 为未保存、有效 Windows UTF-16、损坏、未知 schema/路径编码与受限容量。
    //! stimuli 为 load/save，invariant 为 lossless path 和 replace 失败保留原文件。
    use super::*;
    use std::fs;
    use std::os::windows::ffi::OsStringExt;
    use std::os::windows::fs::OpenOptionsExt;

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
