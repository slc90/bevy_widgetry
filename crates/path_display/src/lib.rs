#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use std::path::{Component, Path, Prefix};

pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::VerbatimDisk(_) => text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned(),
            Prefix::VerbatimUNC(_, _) => text
                .strip_prefix(r"\\?\UNC\")
                .map(|tail| format!(r"\\{tail}"))
                .unwrap_or_else(|| text.into_owned()),
            _ => text.into_owned(),
        },
        _ => text.into_owned(),
    }
}
