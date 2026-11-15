#![allow(clippy::disallowed_macros)]

use bevy_widgetry_path_display::{display_path, simplify_path};
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

#[test]
fn simplifies_native_paths_without_rewriting_the_body() {
    for (input, expected) in [
        (r"\\?\C:\Users\用户\数据", r"C:\Users\用户\数据"),
        (r"\\?\C:\", r"C:\"),
        (r"\\?\UNC\server\share", r"\\server\share"),
        (r"\\?\UNC\server\share\", r"\\server\share\"),
        (
            r"\\?\UNC\服务器\共享\目录 空格\\.\child\..\",
            r"\\服务器\共享\目录 空格\\.\child\..\",
        ),
        (r"\\?\c:\Mixed/分隔\\tail\", r"c:\Mixed/分隔\\tail\"),
        (r"C:\目录 空格\\.\child\..\", r"C:\目录 空格\\.\child\..\"),
        (r"\\server\share\目录", r"\\server\share\目录"),
        (r"relative/目录\\.\..\", r"relative/目录\\.\..\"),
        ("", ""),
        (r"\\.\COM1", r"\\.\COM1"),
        (r"\\?\Volume{1234}\", r"\\?\Volume{1234}\"),
        (r"\\?\pipe\name", r"\\?\pipe\name"),
        (r"relative\a\\?\C:\data", r"relative\a\\?\C:\data"),
        (r"\\?\C:\a\\?\tail", r"C:\a\\?\tail"),
    ] {
        let path = PathBuf::from(input);
        let original = path.as_os_str().to_owned();
        let simplified = simplify_path(&path);
        assert_eq!(simplified.as_os_str(), Path::new(expected).as_os_str());
        assert_eq!(path.as_os_str(), original);
        assert_eq!(
            simplify_path(&simplified).as_os_str(),
            simplified.as_os_str()
        );
        assert_eq!(display_path(&path), expected);
    }
}

#[test]
fn preserves_native_utf16_including_unpaired_surrogates() {
    const UNPAIRED_HIGH_SURROGATE: u16 = 0xD800;
    const UNPAIRED_LOW_SURROGATE: u16 = 0xDC00;
    for (prefix, expected_prefix) in [
        (r"\\?\C:\", r"C:\"),
        (r"\\?\UNC\server\share\", r"\\server\share\"),
        (r"relative\", r"relative\"),
        (r"\\?\Volume{1234}\", r"\\?\Volume{1234}\"),
    ] {
        let body: Vec<_> = [UNPAIRED_HIGH_SURROGATE]
            .into_iter()
            .chain(r"\文件\".encode_utf16())
            .chain([UNPAIRED_LOW_SURROGATE])
            .collect();
        let original: Vec<_> = prefix.encode_utf16().chain(body.iter().copied()).collect();
        let expected: Vec<_> = expected_prefix
            .encode_utf16()
            .chain(body.iter().copied())
            .collect();
        let path = PathBuf::from(OsString::from_wide(&original));
        let simplified = simplify_path(&path);
        assert_eq!(
            simplified.as_os_str().encode_wide().collect::<Vec<_>>(),
            expected
        );
        assert_eq!(path.as_os_str().encode_wide().collect::<Vec<_>>(), original);
    }
}

#[test]
fn preserves_long_native_paths() {
    const LONG_SEGMENT_CHARACTERS: usize = 300;
    let body = format!(
        r"目录\{}\\.\child\..\",
        "文".repeat(LONG_SEGMENT_CHARACTERS)
    );
    for (input_prefix, expected_prefix) in [
        (r"\\?\C:\", r"C:\"),
        (r"\\?\UNC\server\share\", r"\\server\share\"),
    ] {
        let input = PathBuf::from(format!("{input_prefix}{body}"));
        let expected = OsString::from(format!("{expected_prefix}{body}"));
        assert_eq!(simplify_path(&input).as_os_str(), expected);
    }
}

#[test]
fn writes_text_and_configuration_paths_without_verbatim_prefixes() {
    let directory = tempfile::tempdir().unwrap();
    for (index, (input, expected)) in [
        (r"\\?\C:\Users\用户\数据", r"C:\Users\用户\数据"),
        (r"\\?\UNC\server\share\目录", r"\\server\share\目录"),
    ]
    .into_iter()
    .enumerate()
    {
        let path = Path::new(input);
        let text_file = directory.path().join(format!("report-{index}.txt"));
        std::fs::write(&text_file, display_path(path)).unwrap();
        assert_eq!(std::fs::read_to_string(text_file).unwrap(), expected);

        let config_file = directory.path().join(format!("config-{index}.json"));
        let configuration = serde_json::json!({ "path": simplify_path(path) });
        std::fs::write(&config_file, serde_json::to_vec(&configuration).unwrap()).unwrap();
        let restored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(config_file).unwrap()).unwrap();
        assert_eq!(restored["path"].as_str().unwrap(), expected);
        let restored_path: PathBuf = serde_json::from_value(restored["path"].clone()).unwrap();
        assert_eq!(restored_path.as_os_str(), Path::new(expected).as_os_str());
    }
}
