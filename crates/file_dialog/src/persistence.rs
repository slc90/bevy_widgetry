use crate::{
    WidgetryFileDialogFilterId, WidgetryFileDialogSort, WidgetryFileDialogStorageSnapshot,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::unix::ffi::{OsStrExt, OsStringExt};
#[cfg(windows)]
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
    Unix(Vec<u8>),
}

fn invalid(message: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

fn encode(path: &Path) -> io::Result<EncodedPath> {
    if !path.is_absolute() || path.as_os_str().len() > 32768 {
        return Err(invalid("invalid or oversized stored path"));
    }
    #[cfg(windows)]
    {
        let units: Vec<_> = path.as_os_str().encode_wide().collect();
        if units.contains(&0) {
            return Err(invalid("stored path contains NUL"));
        }
        Ok(EncodedPath::Windows(units))
    }
    #[cfg(unix)]
    {
        let units = path.as_os_str().as_bytes();
        if units.contains(&0) {
            return Err(invalid("stored path contains NUL"));
        }
        Ok(EncodedPath::Unix(units.to_vec()))
    }
}

fn decode(encoded: EncodedPath) -> io::Result<Option<PathBuf>> {
    let path = match encoded {
        EncodedPath::Windows(units) => {
            if units.len() > 32768 || units.contains(&0) {
                return Err(invalid("invalid encoded Windows path"));
            }
            #[cfg(windows)]
            {
                Some(PathBuf::from(OsString::from_wide(&units)))
            }
            #[cfg(not(windows))]
            {
                None
            }
        }
        EncodedPath::Unix(units) => {
            if units.len() > 32768 || units.contains(&0) {
                return Err(invalid("invalid encoded Unix path"));
            }
            #[cfg(unix)]
            {
                Some(PathBuf::from(OsString::from_vec(units)))
            }
            #[cfg(not(unix))]
            {
                None
            }
        }
    };
    if path.as_ref().is_some_and(|path| !path.is_absolute()) {
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
        let decoded = prefs
            .pinned
            .into_iter()
            .map(decode)
            .collect::<io::Result<Vec<_>>>()?;
        let visited = prefs.visited.map(decode).transpose()?;
        let picked = prefs.picked.map(decode).transpose()?;
        let unavailable_paths = decoded.iter().filter(|path| path.is_none()).count()
            + usize::from(visited == Some(None))
            + usize::from(picked == Some(None));
        let pinned: Vec<_> = decoded.into_iter().flatten().collect();
        let mut unique = std::collections::BTreeSet::new();
        if pinned.iter().any(|path| !unique.insert(path)) {
            return Err(invalid("duplicate pinned path"));
        }
        result.insert(
            scope,
            WidgetryFileDialogStorageSnapshot {
                last_visited_dir: visited.flatten(),
                last_picked_dir: picked.flatten(),
                show_hidden: prefs.hidden,
                show_system: prefs.system,
                filter: WidgetryFileDialogFilterId(prefs.filter),
                sort,
                pinned,
                unavailable_paths,
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
    //! persistence state 为未保存、有效、损坏、未知 schema 与受限容量。
    //! stimuli 为 load/save，invariant 为 lossless path 和 replace 失败保留原文件。
    use super::*;
    use std::fs;
    #[cfg(windows)]
    use std::os::windows::ffi::OsStringExt;
    #[cfg(windows)]
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

    #[cfg(windows)]
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

    #[cfg(windows)]
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
