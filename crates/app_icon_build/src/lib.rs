//! 为应用的 Windows EXE 嵌入调用方提供的 ICO，无需随程序分发图标文件。
//!
//! 应用在自己的 Cargo build script 中调用 set_exe_icon，失败通过 io::Result 返回。
//! 图标内容变化会触发 build script 重新执行。
//!
//! 相对路径以调用方 CARGO_MANIFEST_DIR 为基准，不依赖 current working directory。
//! 本能力属于构建期，不配置运行时窗口或任务栏图标，也不转换图片格式。

#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

use std::{
    io,
    path::{Path, PathBuf},
};

pub fn set_exe_icon(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref();
    let manifest_dir = std::env::var_os("CARGO_MANIFEST_DIR").ok_or_else(|| {
        io::Error::other("CARGO_MANIFEST_DIR 未设置，请在 Cargo build script 中调用 set_exe_icon")
    })?;
    let path = resolve_icon_path(path, Path::new(&manifest_dir));
    println!("cargo:rerun-if-changed={}", path.display());
    std::fs::File::open(&path)?;
    let path = path
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "ICO 路径必须为有效 UTF-8"))?;
    winresource::WindowsResource::new().set_icon(path).compile()
}

fn resolve_icon_path(path: &Path, manifest_dir: &Path) -> PathBuf {
    manifest_dir.join(path)
}

// 断言用于让 contract 违反时测试失败，生产代码的 disallowed_macros 不适用于此测试 scope。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_relative_to_the_calling_package() {
        let manifest_dir = Path::new("C:/applications/example");
        assert_eq!(
            resolve_icon_path(Path::new("icons/app.ico"), manifest_dir),
            manifest_dir.join("icons/app.ico")
        );
        let absolute = Path::new("D:/icons/app.ico");
        assert_eq!(resolve_icon_path(absolute, manifest_dir), absolute);
    }

    #[test]
    fn missing_icon_returns_not_found() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("missing-widgetry-icon.ico");
        let error = set_exe_icon(path).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
    }
}
