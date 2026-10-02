//! 优化 Gallery 独立进程 startup；首次 state 不预热，readiness 与正常关闭分别观测。

use bevy::prelude::Result;
use bevy_widgetry_test_utils::benchmark::artifact::{Artifact, error};
use serde_json::{Value, json};
use std::fs::{self, File};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// startup 轮询通知延迟包含在结果内，上限通常为一个 polling interval 加调度延迟。
const POLLING: Duration = Duration::from_millis(5);

/// 显式约束样本与 deadline，避免无法终止的外部进程 workload。
struct Options {
    samples: usize,
    timeout: Duration,
    port: u16,
}

/// 保证错误路径释放 child；正常路径先经 BRP 关闭，再验证 exit code。
struct GalleryProcess(Child);

/// Cargo 仅编译 harness；真实 Gallery release binary 另行构建，排除全部编译耗时。
fn main() -> Result {
    let options = options()?;
    let artifact = Artifact::new("startup-rust", "Gallery release; harness bench")?;
    artifact.write_json("options.json", &json!({"samples":options.samples,"timeout_seconds":options.timeout.as_secs(),"port":options.port,"polling_ms":POLLING.as_millis()}))?;
    let executable = build(&artifact)?;
    artifact.executable(&executable, "executable.json")?;
    let mut results = Vec::new();
    for sample in 1..=options.samples {
        let state = artifact.directory.join(format!("sample-{sample}/state"));
        for mode in ["first", "subsequent"] {
            let output = artifact.directory.join(format!("sample-{sample}/{mode}"));
            fs::create_dir_all(&output).map_err(error)?;
            let mut result = json!({"sample":sample,"mode":mode,"status":"failed","startup_ms":null,"polling_ms":5,"readiness":null,"shutdown":null,"error":null});
            let measured = measure(
                &artifact,
                &executable,
                &state,
                &output,
                &options,
                &mut result,
            );
            if let Err(ref failure) = measured {
                result["status"] = json!("failed");
                result["error"] = json!(failure.to_string());
            }
            println!("{result}");
            results.push(result);
            artifact.write_json("samples.json", &json!(results))?;
            measured?;
        }
    }
    Ok(())
}

/// 解析 Rust harness 参数；接受 Cargo 自动附加的 --bench。
fn options() -> Result<Options> {
    let mut options = Options {
        samples: 5,
        timeout: Duration::from_secs(60),
        port: 15982,
    };
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if argument == "--bench" {
            continue;
        }
        let value = arguments
            .next()
            .ok_or_else(|| error(format!("missing value for {argument}")))?;
        match argument.as_str() {
            "--samples" => options.samples = value.parse().map_err(error)?,
            "--timeout-seconds" => {
                options.timeout = Duration::from_secs(value.parse().map_err(error)?)
            }
            "--port" => options.port = value.parse().map_err(error)?,
            _ => return Err(error(format!("unknown startup argument: {argument}"))),
        }
    }
    if !(1..=100).contains(&options.samples)
        || !(1..=300).contains(&options.timeout.as_secs())
        || options.port == 0
    {
        return Err(error(
            "samples must be 1..=100, timeout 1..=300, port nonzero",
        ));
    }
    Ok(options)
}

/// 保存完整 JSON build diagnostics，即使 build 失败也能追溯原始原因。
fn build(artifact: &Artifact) -> Result<PathBuf> {
    let output = Command::new("cargo")
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
    fs::write(artifact.directory.join("build.jsonl"), &output.stdout).map_err(error)?;
    fs::write(artifact.directory.join("build-stderr.log"), &output.stderr).map_err(error)?;
    if !output.status.success() {
        return Err(error(format!(
            "Gallery release build failed: {}",
            output.status
        )));
    }
    for line in String::from_utf8(output.stdout).map_err(error)?.lines() {
        let value: Value = serde_json::from_str(line).map_err(error)?;
        if value["reason"] == "compiler-artifact"
            && value["target"]["name"] == "widget_gallery"
            && let Some(path) = value["executable"].as_str()
        {
            return Ok(PathBuf::from(path));
        }
    }
    Err(error("missing Gallery executable build artifact"))
}

/// 计时仅覆盖 process creation 至 atomic ready.txt；正常 shutdown 与所有准备均排除。
fn measure(
    artifact: &Artifact,
    executable: &Path,
    state: &Path,
    output: &Path,
    options: &Options,
    result: &mut Value,
) -> Result {
    let address = (std::net::Ipv4Addr::LOCALHOST, options.port);
    drop(
        TcpListener::bind(address).map_err(|failure| {
            error(format!("BRP port {} unavailable: {failure}", options.port))
        })?,
    );
    let mut command = Command::new(executable);
    command
        .current_dir(&artifact.workspace)
        .env("GALLERY_STARTUP_BENCH_STATE", state)
        .env("GALLERY_STARTUP_BENCH_OUTPUT", output)
        .env("BRP_EXTRAS_PORT", options.port.to_string())
        .stdout(Stdio::from(
            File::create(output.join("stdout.log")).map_err(error)?,
        ))
        .stderr(Stdio::from(
            File::create(output.join("stderr.log")).map_err(error)?,
        ));
    let start = Instant::now();
    let mut child = GalleryProcess(command.spawn().map_err(error)?);
    let measured = (|| {
        let ready = output.join("ready.txt");
        while !ready.try_exists().map_err(error)? {
            if let Some(status) = child.0.try_wait().map_err(error)? {
                return Err(error(format!("Gallery exited before readiness: {status}")));
            }
            if start.elapsed() >= options.timeout {
                return Err(error("startup readiness timeout"));
            }
            thread::sleep(POLLING);
        }
        result["startup_ms"] = json!(start.elapsed().as_secs_f64() * 1000.0);
        result["readiness"] = json!(fs::read_to_string(ready).map_err(error)?);
        let response: Value = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .build()
            .new_agent()
            .post(format!("http://127.0.0.1:{}", options.port))
            .send_json(json!({"jsonrpc":"2.0","id":1,"method":"brp_extras/shutdown","params":{}}))
            .map_err(error)?
            .body_mut()
            .read_json()
            .map_err(error)?;
        if !response["error"].is_null()
            || response["jsonrpc"] != "2.0"
            || response["id"] != 1
            || response.get("result").is_none()
        {
            return Err(error(format!("BRP shutdown failed: {response}")));
        }
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = child.0.try_wait().map_err(error)? {
                if !status.success() {
                    return Err(error(format!("Gallery shutdown exit: {status}")));
                }
                break;
            }
            if Instant::now() >= deadline {
                return Err(error("normal shutdown timeout"));
            }
            thread::sleep(POLLING);
        }
        result["status"] = json!("ready");
        result["shutdown"] = json!("normal");
        Ok(())
    })();
    if measured.is_err() {
        result["shutdown"] = json!("forced cleanup after failure");
        if let Err(cleanup) = child.cleanup() {
            return Err(error(format!(
                "{}; cleanup failed: {cleanup}",
                measured.err().ok_or_else(|| error("missing failure"))?
            )));
        }
    }
    measured
}

impl GalleryProcess {
    /// 错误路径终止并回收已启动的 Gallery，保留原始 stdout / stderr 文件。
    fn cleanup(&mut self) -> Result {
        if self.0.try_wait().map_err(error)?.is_none() {
            self.0.kill().map_err(error)?;
            self.0.wait().map_err(error)?;
        }
        Ok(())
    }
}

impl Drop for GalleryProcess {
    /// artifact 写入等非正常返回路径同样必须释放 child，失败输出明确诊断。
    fn drop(&mut self) {
        if let Err(failure) = self.cleanup() {
            eprintln!("Gallery cleanup failed: {failure}");
        }
    }
}
