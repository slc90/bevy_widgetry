#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use std::{fs, path::Path, process::Command};

fn fixture(directory: &Path) {
    let gallery = Path::new(env!("CARGO_MANIFEST_DIR"));
    let build = gallery
        .join("build.rs")
        .to_string_lossy()
        .replace('\\', "/");
    let library = gallery
        .join("../crates/app_icon_build")
        .to_string_lossy()
        .replace('\\', "/");
    fs::create_dir_all(directory.join("src/assets/icons")).unwrap();
    fs::write(
        directory.join("Cargo.toml"),
        format!(
            r#"
[workspace]
[package]
name = "icon_build_probe"
version = "0.1.0"
edition = "2024"
build = "{build}"
[build-dependencies]
bevy_widgetry_app_icon_build = {{ path = "{library}" }}
tracing = "0.1.44"
tracing-subscriber = "0.3.23"
"#
        ),
    )
    .unwrap();
    fs::write(directory.join("src/main.rs"), "fn main() {}\n").unwrap();
}

#[test]
fn cargo_build_entry_reports_missing_invalid_and_compiler_failures_without_backtrace_environment() {
    let gallery = Path::new(env!("CARGO_MANIFEST_DIR"));
    let directory = tempfile::tempdir().unwrap();
    fixture(directory.path());
    let icon = directory
        .path()
        .join("src/assets/icons/widget_gallery_exe.ico");
    let target = gallery.join("../target/error-57-build-fixture");
    for scenario in ["missing", "invalid", "compiler", "valid"] {
        match scenario {
            "invalid" => fs::write(&icon, b"invalid ICO").unwrap(),
            "compiler" | "valid" => {
                fs::copy(
                    gallery.join("src/assets/icons/widget_gallery_exe.ico"),
                    &icon,
                )
                .unwrap();
            }
            _ => {}
        }
        let mut command = Command::new(env!("CARGO"));
        command
            .args(["build", "--offline", "--manifest-path"])
            .arg(directory.path().join("Cargo.toml"))
            .arg("--target-dir")
            .arg(&target)
            .env_remove("RUST_BACKTRACE")
            .env_remove("RUST_LIB_BACKTRACE")
            .env_remove("BEVY_BACKTRACE")
            .env_remove("RUST_LOG")
            .env_remove("RC_PATH");
        if scenario == "compiler" {
            command.env("RC_PATH", directory.path().join("missing-rc.exe"));
        }
        let output = command.output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if scenario == "valid" {
            assert!(output.status.success(), "{stderr}");
            let executable = target.join("debug/icon_build_probe.exe");
            let module = winsafe::HINSTANCE::LoadLibrary(executable.to_str().unwrap()).unwrap();
            let icon = module
                .FindResource(
                    winsafe::IdStr::Id(1),
                    winsafe::RtStr::Rt(winsafe::co::RT::GROUP_ICON),
                )
                .unwrap();
            assert!(module.SizeofResource(&icon).unwrap() > 0);
        } else {
            assert!(!output.status.success(), "{scenario}");
            assert_eq!(stderr.matches("phase=\"source\"").count(), 1, "{stderr}");
            assert_eq!(stderr.matches("phase=unhandled").count(), 1, "{stderr}");
            let source = stderr
                .lines()
                .find(|line| line.contains("phase=\"source\""))
                .unwrap();
            let id = source
                .split("error_id=")
                .nth(1)
                .unwrap()
                .split_whitespace()
                .next()
                .unwrap();
            let terminal = stderr
                .lines()
                .find(|line| line.contains("phase=unhandled"))
                .unwrap();
            assert!(terminal.contains(&format!("error_id={id} ")), "{stderr}");
            assert!(!source.contains("backtrace="), "{source}");
            assert!(
                source.contains(if scenario == "missing" {
                    "icon.build.read"
                } else {
                    "icon.build.compile"
                }),
                "{source}"
            );
            assert!(
                stderr.contains("error_id=") && stderr.contains("widget_gallery_exe.ico"),
                "{stderr}"
            );
            assert!(
                stderr.contains("backtrace=") && !stderr.contains("backtrace=disabled backtrace"),
                "{stderr}"
            );
            assert!(!stderr.contains("panicked at"), "{stderr}");
        }
    }
}
