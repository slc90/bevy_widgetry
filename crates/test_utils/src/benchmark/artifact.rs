use bevy::prelude::{BevyError, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Artifact {
    pub directory: PathBuf,
    pub workspace: PathBuf,
}

pub fn error(error: impl std::fmt::Display) -> BevyError {
    BevyError::error(error.to_string())
}

fn hardware() -> Result<Value> {
    let connection = wmi::WMIConnection::new().map_err(error)?;
    let mut result = serde_json::Map::new();
    for (name, query) in [
        (
            "os",
            "SELECT Caption,Version,BuildNumber FROM Win32_OperatingSystem",
        ),
        (
            "cpu",
            "SELECT Name,NumberOfCores,NumberOfLogicalProcessors FROM Win32_Processor",
        ),
        (
            "gpu",
            "SELECT Name,DriverVersion FROM Win32_VideoController",
        ),
    ] {
        let values: Vec<HashMap<String, Value>> = connection.raw_query(query).map_err(error)?;
        result.insert(name.into(), json!(values));
    }
    Ok(Value::Object(result))
}

impl Artifact {
    pub fn new(owner: &str, profile: &str) -> Result<Self> {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .map_err(error)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(error)?
            .as_millis();
        let parent = workspace.join("target/benchmark");
        fs::create_dir_all(&parent).map_err(error)?;
        let directory = parent.join(format!("{owner}-{timestamp}-{}", std::process::id()));
        fs::create_dir(&directory).map_err(error)?;
        let artifact = Self {
            directory,
            workspace,
        };
        artifact.write_json("environment.json", &json!({
            "measured_at_unix_ms": timestamp,
            "command": std::env::args_os().map(|value| value.to_string_lossy().into_owned()).collect::<Vec<_>>(),
            "commit": artifact.command("git", &["rev-parse", "HEAD"] )?,
            "working_tree": artifact.command("git", &["status", "--porcelain"] )?,
            "rust": artifact.command("rustc", &["-Vv"] )?,
            "hardware": hardware()?, "profile": profile,
            "features": "workspace defaults; cargo --locked",
            "rustflags": std::env::var_os("RUSTFLAGS").map(|s| s.to_string_lossy().into_owned()),
            "cargo_encoded_rustflags": std::env::var_os("CARGO_ENCODED_RUSTFLAGS").map(|s| s.to_string_lossy().into_owned()),
            "rust_log": std::env::var_os("RUST_LOG").map(|s| s.to_string_lossy().into_owned()),
            "tool": if owner.ends_with("-criterion") { "Criterion 0.8.2; flat sampling; sample_size=20; warmup=100ms; measurement=500ms; defaults overridable by Criterion CLI" } else { "Rust GUI benchmark harness; warmup configured by scenario" }
        }))?;
        let patch = artifact.command("git", &["diff", "HEAD", "--binary"])?;
        fs::write(artifact.directory.join("tracked.patch"), patch).map_err(error)?;
        let files = artifact.command(
            "git",
            &[
                "-c",
                "core.quotepath=false",
                "ls-files",
                "--others",
                "--exclude-standard",
                "-z",
            ],
        )?;
        let mut hashes = Vec::new();
        for file in files
            .split('\0')
            .filter(|file| !file.is_empty() && !file.ends_with("-进度.md"))
        {
            let source = artifact.workspace.join(file);
            let target = artifact.directory.join("untracked").join(file);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(error)?;
            }
            fs::copy(&source, &target).map_err(error)?;
            hashes.push(json!({ "path": file, "sha256": Self::hash(&target)? }));
        }
        artifact.write_json("untracked.json", &json!(hashes))?;
        artifact.executable(&std::env::current_exe().map_err(error)?, "harness.json")?;
        println!("Benchmark artifacts: {}", artifact.directory.display());
        Ok(artifact)
    }

    fn command(&self, program: &str, args: &[&str]) -> Result<String> {
        let output = Command::new(program)
            .args(args)
            .current_dir(&self.workspace)
            .output()
            .map_err(error)?;
        if !output.status.success() {
            return Err(error(format!(
                "{program} {args:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        String::from_utf8(output.stdout).map_err(error)
    }

    pub fn write_json(&self, name: &str, value: &Value) -> Result {
        let mut file = File::create(self.directory.join(name)).map_err(error)?;
        file.write_all(&serde_json::to_vec_pretty(value).map_err(error)?)
            .map_err(error)
    }

    pub fn executable(&self, path: &Path, name: &str) -> Result {
        self.write_json(name, &json!({"path": path, "sha256": Self::hash(path)?}))
    }

    fn hash(path: &Path) -> Result<String> {
        let mut file = File::open(path).map_err(error)?;
        let mut digest = Sha256::new();
        let mut buffer = [0; 65536];
        loop {
            let size = file.read(&mut buffer).map_err(error)?;
            if size == 0 {
                break;
            }
            digest.update(&buffer[..size]);
        }
        Ok(format!("{:x}", digest.finalize()))
    }
}
