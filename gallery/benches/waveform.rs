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
    let algorithm =
        std::env::var("GALLERY_WAVEFORM_BENCH_ALGORITHM").unwrap_or_else(|_| "MinMax".into());
    if !["MinMax", "Lttb"].contains(&algorithm.as_str()) {
        return Err(error("Invalid waveform benchmark algorithm"));
    }
    let scenario =
        std::env::var("GALLERY_WAVEFORM_BENCH_SCENARIO").unwrap_or_else(|_| "Live".into());
    if !["Live", "Replay"].contains(&scenario.as_str()) {
        return Err(error("Invalid waveform benchmark scenario"));
    }
    let port = 15983;
    drop(TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).map_err(error)?);
    let artifact = Artifact::new(
        "waveform-gui",
        "Gallery release; desktop_app; DX12; 1920x1080; stress root 1600x640 with toolbar and time axis",
    )?;
    let built = Command::new("cargo")
        .args([
            "build",
            "--locked",
            "--release",
            "--workspace",
            "--bin",
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
    let mut replay_frames = None;
    if scenario == "Replay" {
        let path = std::env::var_os("GALLERY_WAVEFORM_REPLAY")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| artifact.workspace.join("gallery/data/waveform/replay.widx"));
        let recording = bevy_widgetry_waveform_file::Recording::open(&path).map_err(error)?;
        let metadata = recording.metadata();
        if let bevy_widgetry::waveform::WaveformSessionKind::Replay { total_frames } = metadata.kind
        {
            replay_frames = Some(total_frames);
        }
        artifact.executable(&path, "recording.json")?;
        artifact.write_json("replay.json", &json!({"path":path,"channels":metadata.channel_count,"rate":metadata.sample_rate,"kind":format!("{:?}",metadata.kind),"algorithms":"initial algorithm for 30s, then the other for 30s", "scenario":"isolated disk Replay; no Live producer; random warm file I/O may use OS cache","query_time":"worker query including I/O and reduction","io_time":"seek/read_exact only; includes OS cache","cpu_time":"query minus measured I/O; includes validation and allocation","observable":"seek intent to GPU readback upper bound; excludes BRP RTT and PNG; not OS presentation"}))?;
    }
    let output = artifact.directory.join("run");
    fs::create_dir_all(&output).map_err(error)?;
    artifact.write_json("scenario.json", &json!({"scenario":scenario,"algorithm":algorithm,"live_load":if scenario == "Live" { Some(json!({"sample_rate":64000,"channels":64,"visible_duration_seconds":10,"waveform_root":[1600,640]})) } else { None },"replay_load":"see replay.json when selected","viewport_height":"measured in frames.csv; excludes toolbar and time axis","window_config":"recorded in window.json","update_mode":"desktop_app","warmup_seconds":12,"duration_seconds":90,"burst_hold_ms":if scenario == "Live" {100} else {0},"clock":"Instant","observable":"generation/seek intent to GPU readback upper bound; no PNG encoding, BRP RTT or OS presentation"}))?;
    let mut child = Process(
        Command::new(&executable)
            .current_dir(&artifact.workspace)
            .env("BRP_EXTRAS_PORT", port.to_string())
            .env("GALLERY_WAVEFORM_BENCH_OUTPUT", &output)
            .env("GALLERY_WAVEFORM_BENCH_ALGORITHM", &algorithm)
            .env("GALLERY_WAVEFORM_BENCH_SCENARIO", &scenario)
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
            &output.join("navigation/ready.txt"),
            Duration::from_secs(60),
        )?;
        let windows = rpc(
            port,
            "world.query",
            json!({"data":{"components":["bevy_window::window::Window"]}}),
        )?;
        artifact.write_json("window.json", &windows)?;
        let (nav_x, nav_y, _) = named_node(port, "WaveformNav")?;
        rpc(
            port,
            "brp_extras/move_mouse",
            json!({"position":[nav_x,nav_y]}),
        )?;
        rpc(port, "brp_extras/click_mouse", json!({"button":"Left"}))?;
        if scenario == "Replay" {
            let inputs = replay_inputs(
                port,
                &algorithm,
                replay_frames.ok_or_else(|| error("Replay metadata missing"))?,
            )?;
            artifact.write_json("inputs.json", &inputs)?;
        }
        wait(
            &mut child,
            &output.join("done.txt"),
            Duration::from_secs(120),
        )?;
        release_pointer(port)?;
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
    if measured.is_err() {
        let pointer = release_pointer(port);
        let shutdown = rpc(port, "brp_extras/shutdown", json!({}));
        artifact.write_json("cleanup.json",&json!({"pointer_error":pointer.err().map(|error|error.to_string()),"shutdown_error":shutdown.err().map(|error|error.to_string())}))?;
    }
    artifact.write_json("result.json", &json!({"status":if measured.is_ok(){"complete"}else{"failed"},"error":measured.as_ref().err().map(ToString::to_string),"shutdown":if measured.is_ok(){"normal"}else{"see cleanup.json; Process drop waits or kills the owned child"}}))?;
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

fn rpc(port: u16, method: &str, params: Value) -> Result<Value> {
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
    Ok(response["result"].clone())
}

fn pointer_idle(port: u16) -> Result {
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let state = rpc(
            port,
            "brp_extras/pointer_control",
            json!({"action":"status"}),
        )?;
        if !state["last_error"].is_null() {
            return Err(error(format!("pointer failed: {state}")));
        }
        if state["busy"] == false {
            return Ok(());
        }
        if Instant::now() >= until {
            return Err(error("pointer input timed out"));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn named_node(port: u16, name: &str) -> Result<(f64, f64, f64)> {
    let rows = rpc(
        port,
        "world.query",
        json!({"data":{"components":["bevy_ecs::name::Name","bevy_ui::ui_node::ComputedNode","bevy_ui::ui_transform::UiGlobalTransform"]}}),
    )?;
    let rows = rows
        .as_array()
        .ok_or_else(|| error("world.query did not return rows"))?;
    for row in rows {
        let components = &row["components"];
        if components["bevy_ecs::name::Name"] != name {
            continue;
        }
        let node = &components["bevy_ui::ui_node::ComputedNode"];
        let width = node["size"][0]
            .as_f64()
            .ok_or_else(|| error("node width missing"))?;
        if width <= 0.0 {
            continue;
        }
        let inverse = node["inverse_scale_factor"]
            .as_f64()
            .ok_or_else(|| error("node scale missing"))?;
        let transform = &components["bevy_ui::ui_transform::UiGlobalTransform"];
        return Ok((
            transform[4]
                .as_f64()
                .ok_or_else(|| error("node x missing"))?
                * inverse,
            transform[5]
                .as_f64()
                .ok_or_else(|| error("node y missing"))?
                * inverse,
            width * inverse,
        ));
    }
    Err(error(format!("visible node not found: {name}")))
}

fn replay_inputs(port: u16, initial_algorithm: &str, total_frames: u64) -> Result<Value> {
    pointer_idle(port)?;
    let until = Instant::now() + Duration::from_secs(60);
    println!("请将 Gallery 切至前台并保持焦点，以进行真实 Slider 拖动测量");
    loop {
        let windows = rpc(
            port,
            "world.query",
            json!({"data":{"components":["bevy_window::window::Window"]}}),
        )?;
        if windows.as_array().is_some_and(|rows| {
            rows.iter()
                .any(|row| row["components"]["bevy_window::window::Window"]["focused"] == true)
        }) {
            break;
        }
        if Instant::now() >= until {
            return Err(error("Gallery did not receive OS focus"));
        }
        thread::sleep(Duration::from_millis(100));
    }
    let (x, y, _) = named_node(port, "WaveformModeDisk")?;
    rpc(port, "brp_extras/move_mouse", json!({"position":[x,y]}))?;
    rpc(port, "brp_extras/click_mouse", json!({"button":"Left"}))?;
    pointer_idle(port)?;
    thread::sleep(Duration::from_secs(2));
    let (x, y, width) = named_node(port, "WaveformReplayProgress")?;
    thread::sleep(Duration::from_secs(12));
    let start = Instant::now();
    let mut index = 0;
    let mut switched = false;
    while start.elapsed() < Duration::from_secs(60) {
        if !switched && start.elapsed() >= Duration::from_secs(30) {
            let (combo_x, combo_y, _) = named_node(port, "WaveformAlgorithm")?;
            rpc(
                port,
                "brp_extras/move_mouse",
                json!({"position":[combo_x,combo_y]}),
            )?;
            rpc(port, "brp_extras/click_mouse", json!({"button":"Left"}))?;
            pointer_idle(port)?;
            rpc(
                port,
                "brp_extras/send_keys",
                json!({"keys":[if initial_algorithm == "MinMax" {"End"} else {"Home"}]}),
            )?;
            rpc(port, "brp_extras/send_keys", json!({"keys":["Enter"]}))?;
            switched = true;
        }
        let (a, b) = if index % 2 == 0 {
            (-0.4, 0.4)
        } else {
            (0.4, -0.4)
        };
        rpc(
            port,
            "brp_extras/drag_mouse",
            json!({"button":"Left","start":[x+width*a,y],"end":[x+width*b,y],"frames":18}),
        )?;
        pointer_idle(port)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let rows = rpc(
                port,
                "world.query",
                json!({"data":{"components":["bevy_ecs::name::Name","widget_gallery::pages::waveform::WaveformDemoState"]}}),
            )?;
            let root = rows
                .as_array()
                .and_then(|rows| {
                    rows.iter().find(|row| {
                        row["components"]["bevy_ecs::name::Name"] == "DiskReplayWaveform"
                    })
                })
                .ok_or_else(|| error("Replay demo state missing"))?;
            let position = root["components"]["widget_gallery::pages::waveform::WaveformDemoState"]
                ["producer_frame"]
                .as_u64()
                .ok_or_else(|| error("Replay target missing"))? as f64
                / total_frames as f64;
            if b > 0.0 && position < 0.75 || b < 0.0 && position > 0.25 {
                return Err(error(
                    "Replay drag was accepted without reaching its target; verify OS focus",
                ));
            }
            let state = &root["components"]["widget_gallery::pages::waveform::WaveformDemoState"];
            let expected_algorithm = if switched {
                if initial_algorithm == "MinMax" {
                    "Lttb"
                } else {
                    "MinMax"
                }
            } else {
                initial_algorithm
            };
            if state["algorithm"] == expected_algorithm
                && state["preview"] == false
                && state["displayed_frame"] == state["producer_frame"]
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err(error(format!(
                    "Replay final snapshot did not converge: {state}"
                )));
            }
            thread::sleep(Duration::from_millis(10));
        }
        index += 1;
    }
    release_pointer(port)?;
    Ok(
        json!({"committed_drags":index,"elapsed_seconds":start.elapsed().as_secs_f64(),"initial_algorithm":initial_algorithm,"switched_algorithm":switched,"verified":"Slider endpoint, selected algorithm, preview=false and displayed_frame=producer_frame after each release"}),
    )
}

fn release_pointer(port: u16) -> Result {
    rpc(
        port,
        "brp_extras/pointer_control",
        json!({"action":"release"}),
    )?;
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let state = rpc(
            port,
            "brp_extras/pointer_control",
            json!({"action":"status"}),
        )?;
        if state["phase"] == "inactive" {
            if !state["last_error"].is_null() {
                return Err(error(format!("pointer cleanup failed: {state}")));
            }
            break;
        }
        if Instant::now() >= until {
            return Err(error("pointer cleanup timed out"));
        }
        thread::sleep(Duration::from_millis(10));
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
