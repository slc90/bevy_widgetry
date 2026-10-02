//! release Gallery 独立进程、真实 sidebar mouse input 与完整 GUI pipeline 的持续测量。

use bevy::prelude::Result;
use bevy_widgetry_test_utils::benchmark::artifact::{Artifact, error};
use serde_json::{Value, json};
use std::fs::{self, File};
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct Process(Child);

fn main() -> Result {
    let port = 15983;
    drop(TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).map_err(error)?);
    let artifact = Artifact::new(
        "waveform-gui",
        "Gallery release; desktop_app; DX12; 1920x1080; stress 1600x640",
    )?;
    let built = Command::new("cargo")
        .args([
            "build",
            "--locked",
            "--release",
            "-p",
            "widget_gallery",
            "--message-format=json",
        ])
        .current_dir(&artifact.workspace)
        .output()
        .map_err(error)?;
    fs::write(artifact.directory.join("build.jsonl"), &built.stdout).map_err(error)?;
    if !built.status.success() {
        return Err(error(String::from_utf8_lossy(&built.stderr)));
    }
    let mut executable = None;
    for line in String::from_utf8(built.stdout).map_err(error)?.lines() {
        let value: Value = serde_json::from_str(line).map_err(error)?;
        if value["reason"] == "compiler-artifact"
            && value["target"]["name"] == "widget_gallery"
            && let Some(path) = value["executable"].as_str()
        {
            executable = Some(std::path::PathBuf::from(path));
        }
    }
    let executable = executable.ok_or_else(|| error("missing Gallery executable"))?;
    artifact.executable(&executable, "executable.json")?;
    let output = artifact.directory.join("run");
    let startup = artifact.directory.join("startup");
    fs::create_dir_all(&output).map_err(error)?;
    fs::create_dir_all(&startup).map_err(error)?;
    artifact.write_json("scenario.json", &json!({"sample_rate":64000,"channels":64,"duration_seconds":10,"viewport":[1600,640],"window":[1920,1080],"scale_factor":1,"update_mode":"desktop_app","warmup_seconds":12,"duration_seconds":90,"burst_hold_ms":100,"navigation_input":{"position":[80,568],"button":"Left"},"clock":"Instant","observable":"generation-to-GPU-readback upper bound; no PNG encoding or BRP RTT"}))?;
    let mut child = Process(
        Command::new(&executable)
            .current_dir(&artifact.workspace)
            .env("BRP_EXTRAS_PORT", port.to_string())
            .env("GALLERY_WAVEFORM_BENCH_OUTPUT", &output)
            .env("GALLERY_STARTUP_BENCH_OUTPUT", &startup)
            .env(
                "GALLERY_STARTUP_BENCH_STATE",
                artifact.directory.join("state"),
            )
            .stdout(Stdio::from(
                File::create(output.join("stdout.log")).map_err(error)?,
            ))
            .stderr(Stdio::from(
                File::create(output.join("stderr.log")).map_err(error)?,
            ))
            .spawn()
            .map_err(error)?,
    );
    let measured = (|| -> Result {
        wait(
            &mut child,
            &startup.join("ready.txt"),
            Duration::from_secs(60),
        )?;
        rpc(
            port,
            "brp_extras/move_mouse",
            json!({"position":[80.0,568.0]}),
        )?;
        rpc(port, "brp_extras/click_mouse", json!({"button":"Left"}))?;
        wait(
            &mut child,
            &output.join("done.txt"),
            Duration::from_secs(120),
        )?;
        rpc(port, "brp_extras/shutdown", json!({}))?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while child.0.try_wait().map_err(error)?.is_none() {
            if Instant::now() >= deadline {
                return Err(error("normal shutdown timeout"));
            }
            thread::sleep(Duration::from_millis(10));
        }
        let status = child.0.wait().map_err(error)?;
        if !status.success() {
            return Err(error(format!("Gallery exit: {status}")));
        }
        Ok(())
    })();
    artifact.write_json("result.json", &json!({"status":if measured.is_ok(){"complete"}else{"failed"},"error":measured.as_ref().err().map(ToString::to_string),"shutdown":if measured.is_ok(){"normal"}else{"forced cleanup"}}))?;
    println!("GUI artifact: {}", artifact.directory.display());
    measured
}

fn wait(process: &mut Process, marker: &std::path::Path, timeout: Duration) -> Result {
    let deadline = Instant::now() + timeout;
    while !marker.try_exists().map_err(error)? {
        if let Some(status) = process.0.try_wait().map_err(error)? {
            return Err(error(format!("Gallery exited before marker: {status}")));
        }
        if Instant::now() >= deadline {
            return Err(error(format!("timeout waiting for {}", marker.display())));
        }
        thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

fn rpc(port: u16, method: &str, params: Value) -> Result {
    let response: Value = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .build()
        .new_agent()
        .post(format!("http://127.0.0.1:{port}"))
        .send_json(json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .map_err(error)?
        .body_mut()
        .read_json()
        .map_err(error)?;
    if !response["error"].is_null() || response["id"] != 1 || response.get("result").is_none() {
        return Err(error(format!("{method} failed: {response}")));
    }
    Ok(())
}

impl Drop for Process {
    fn drop(&mut self) {
        let result = (|| -> std::io::Result<()> {
            if self.0.try_wait()?.is_none() {
                self.0.kill()?;
                self.0.wait()?;
            }
            Ok(())
        })();
        if let Err(failure) = result {
            eprintln!("Gallery cleanup failed: {failure}");
        }
    }
}
