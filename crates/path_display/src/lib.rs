#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path, PathBuf, Prefix};

pub fn simplify_path(path: &Path) -> PathBuf {
    let Some((prefix, replacement)) = prefix_replacement(path) else {
        return path.to_path_buf();
    };
    let wide: Vec<_> = replacement
        .encode_utf16()
        .chain(
            path.as_os_str()
                .encode_wide()
                .skip(prefix.encode_utf16().count()),
        )
        .collect();
    PathBuf::from(OsString::from_wide(&wide))
}

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    match prefix_replacement(path) {
        Some((prefix, replacement)) => text
            .strip_prefix(prefix)
            .map(|tail| format!("{replacement}{tail}"))
            .unwrap_or_else(|| text.into_owned()),
        None => text.into_owned(),
    }
}

fn prefix_replacement(path: &Path) -> Option<(&'static str, &'static str)> {
    let Component::Prefix(prefix) = path.components().next()? else {
        return None;
    };
    let (prefix, replacement) = match prefix.kind() {
        Prefix::VerbatimDisk(_) => (r"\\?\", ""),
        Prefix::VerbatimUNC(_, _) => (r"\\?\UNC\", r"\\"),
        _ => return None,
    };
    path.as_os_str()
        .as_encoded_bytes()
        .starts_with(prefix.as_bytes())
        .then_some((prefix, replacement))
}
