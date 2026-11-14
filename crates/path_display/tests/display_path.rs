#![allow(clippy::disallowed_macros)]

use bevy_widgetry_path_display::display_path;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

#[test]
fn simplifies_only_verbatim_disk_and_unc_prefixes() {
    for (input, expected) in [
        (r"\\?\C:\Users\用户\数据", r"C:\Users\用户\数据"),
        (r"\\?\C:\", r"C:\"),
        (r"\\?\UNC\server\share", r"\\server\share"),
        (r"\\?\UNC\server\share\", r"\\server\share\"),
        (r"\\?\UNC\server\share\目录", r"\\server\share\目录"),
        (r"C:\Users\用户\数据", r"C:\Users\用户\数据"),
        (r"\\server\share\目录", r"\\server\share\目录"),
        (r"relative\目录", r"relative\目录"),
        ("", ""),
        (r"\\.\COM1", r"\\.\COM1"),
        (r"\\?\Volume{1234}\", r"\\?\Volume{1234}\"),
        (r"\\?\pipe\name", r"\\?\pipe\name"),
        (r"relative\a\\?\C:\data", r"relative\a\\?\C:\data"),
    ] {
        assert_eq!(display_path(Path::new(input)), expected, "{input:?}");
    }
}

#[test]
fn preserves_path_body_without_normalizing_components() {
    for (input, expected) in [
        (
            r"\\?\c:\目录 空格\\.\child\..\",
            r"c:\目录 空格\\.\child\..\",
        ),
        (r"\\?\C:\Mixed/分隔\\tail\", r"C:\Mixed/分隔\\tail\"),
        (
            r"\\?\UNC\服务器\共享\目录 空格\\.\child\..\",
            r"\\服务器\共享\目录 空格\\.\child\..\",
        ),
        (r"C:\目录 空格\\.\child\..\", r"C:\目录 空格\\.\child\..\"),
        (r"relative/目录\\.\..\", r"relative/目录\\.\..\"),
        (r"\\?\C:\a\\?\tail", r"C:\a\\?\tail"),
    ] {
        let path = PathBuf::from(input);
        let original = path.as_os_str().to_owned();
        assert_eq!(display_path(&path), expected, "{input:?}");
        assert_eq!(path.as_os_str(), original);
    }
}

#[test]
fn does_not_truncate_long_paths() {
    const LONG_SEGMENT_CHARACTERS: usize = 300;
    let body = format!(r"C:\{}\目录 空格\", "文".repeat(LONG_SEGMENT_CHARACTERS));
    assert_eq!(display_path(Path::new(&format!(r"\\?\{body}"))), body);
    assert_eq!(display_path(Path::new(&body)), body);
    let unc_body = format!(r"server\share\{}\", "文".repeat(LONG_SEGMENT_CHARACTERS));
    assert_eq!(
        display_path(Path::new(&format!(r"\\?\UNC\{unc_body}"))),
        format!(r"\\{unc_body}")
    );
}

#[test]
fn uses_lossy_unicode_without_changing_the_original_path() {
    const UNPAIRED_HIGH_SURROGATE: u16 = 0xD800;
    for (prefix, displayed_prefix) in [
        (r"\\?\C:\", r"C:\"),
        (r"\\?\UNC\server\share\", r"\\server\share\"),
        (r"relative\", r"relative\"),
        (r"\\?\Volume{1234}\", r"\\?\Volume{1234}\"),
    ] {
        let wide: Vec<_> = prefix
            .encode_utf16()
            .chain([UNPAIRED_HIGH_SURROGATE])
            .chain(r"\文件".encode_utf16())
            .collect();
        let original = OsString::from_wide(&wide);
        let path = PathBuf::from(&original);
        assert_eq!(
            display_path(&path),
            format!("{displayed_prefix}\u{fffd}\\文件")
        );
        assert_eq!(path.as_os_str(), original);
    }
}
