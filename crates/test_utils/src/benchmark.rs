//! Criterion CPU benchmark 与独立逐次 latency / entity 观测；不进入库生产路径。

pub mod artifact;

use artifact::{Artifact, error};
use bevy::prelude::*;
use bevy::text::TextLayoutInfo;
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use criterion::{Criterion, SamplingMode};
use serde_json::json;
use std::fs::{self, File};
use std::hint::black_box;
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant};

/// 连续 workload 的预热次数；lifecycle 每个样本都重新 setup。
const WARMUP: usize = 20;

/// 200 次支持描述本次运行的 P95；不从这些样本宣称可靠 P99。
const SAMPLES: usize = 200;

/// Criterion 管理统计与 baseline；独立 artifact 保存逐次 latency 与环境。
pub struct Harness {
    /// Criterion 的 filter 同时控制额外采样，跳过的场景不创建 fixture。
    criterion: Criterion,
    /// 本次完整运行的来源与失败状态。
    artifact: Artifact,
    /// 逐次操作统计，不将 Criterion batch time 解释成 P95。
    csv: File,
    /// 每次 operation 的原始 latency 支持审计 tail 统计。
    raw: File,
    /// list / test / profile / load 模式遵循 Criterion 语义，不追加独立计时。
    observe: bool,
}

/// 测量真实 operation，setup、entity 观测与 drop 都排除；lifecycle 只采样 20 次并不报告 P95。
/// 连续 fixture 不重置，调用方必须定义每轮均有实际工作的有界 workload。
pub fn run<F>(
    harness: &mut Harness,
    name: &str,
    lifecycle: bool,
    mut setup: impl FnMut() -> Result<F>,
    mut operation: impl FnMut(&mut F, usize) -> Result,
    mut entities: impl FnMut(&mut F) -> Result<u32>,
) -> Result {
    let mut executed = false;
    let mut fixture = None;
    let mut index = 0usize;
    let artifact = &harness.artifact;
    let (group_name, operation_name) = name
        .rsplit_once('/')
        .ok_or_else(|| error("benchmark name must contain group/operation"))?;
    let mut group = harness.criterion.benchmark_group(group_name);
    group.sampling_mode(SamplingMode::Flat);
    group.bench_function(operation_name, |bencher| {
        executed = true;
        bencher.iter_custom(|iterations| {
            let mut measured = Duration::ZERO;
            for _ in 0..iterations {
                if lifecycle || fixture.is_none() {
                    fixture = Some(required(artifact, name, setup()));
                }
                if let Some(fixture) = fixture.as_mut() {
                    let start = Instant::now();
                    required(artifact, name, operation(fixture, index));
                    measured += start.elapsed();
                    black_box(required(artifact, name, entities(fixture)));
                }
                index = index.wrapping_add(1);
            }
            measured
        });
    });
    group.finish();
    drop(fixture);
    if !executed || !harness.observe {
        return Ok(());
    }
    let result = sample(harness, name, lifecycle, setup, operation, entities);
    if let Err(ref failure) = result {
        harness.artifact.write_json(
            "status.json",
            &json!({"status":"failed", "scenario":name,"error":failure.to_string()}),
        )?;
    }
    result
}

/// Criterion baseline 参数的长短形式统一读取，目录名称必须在第三方 I/O 前校验。
fn baseline_argument(arguments: &[String], flag: &str, short: Option<&str>) -> Option<String> {
    arguments
        .iter()
        .enumerate()
        .take_while(|(_, argument)| argument.as_str() != "--")
        .find_map(|(index, argument)| {
            if argument == flag {
                return arguments.get(index + 1).cloned();
            }
            if let Some(value) = argument.strip_prefix(&format!("{flag}=")) {
                return Some(value.into());
            }
            let requested = short.and_then(|prefix| prefix.chars().nth(1))?;
            let cluster = argument
                .strip_prefix('-')
                .filter(|value| !value.starts_with('-'))?;
            for (position, option) in cluster.char_indices() {
                match option {
                    // Criterion 0.8 的这些短参数不消费后续值，允许继续读取组合 flags。
                    'v' | 'n' | 'h' | 'V' => continue,
                    's' | 'b' if option == requested => {
                        let value = &cluster[position + option.len_utf8()..];
                        if value.is_empty() {
                            return arguments.get(index + 1).cloned();
                        }
                        return Some(value.strip_prefix('=').unwrap_or(value).into());
                    }
                    // color / 另一 baseline option 消费余下的整个值，未知 option 由 Clap 拒绝。
                    _ => return None,
                }
            }
            None
        })
}

/// Windows 下大小写变体与 Criterion 内部目录同样冲突；比较和保存均不得使用保留名称。
fn validate_baseline_name(baseline: &str) -> Result {
    if baseline.is_empty()
        || !baseline
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(error(
            "baseline name must contain only ASCII letters, digits, underscore or hyphen",
        ));
    }
    if ["new", "change", "report", "profile"]
        .iter()
        .any(|reserved| baseline.eq_ignore_ascii_case(reserved))
    {
        return Err(error(format!(
            "baseline name {baseline} is reserved by Criterion"
        )));
    }
    Ok(())
}

/// named baseline 必须是新名称；比较使用 Criterion --baseline，禁止保存时覆盖已有结果。
fn ensure_new_baseline(directory: &Path, baseline: &str) -> Result {
    validate_baseline_name(baseline)?;
    if !directory.try_exists().map_err(error)? {
        return Ok(());
    }
    let mut pending = vec![directory.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).map_err(error)? {
            let entry = entry.map_err(error)?;
            if !entry.file_type().map_err(error)?.is_dir() {
                continue;
            }
            let path = entry.path();
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(baseline))
                && path.join("estimates.json").try_exists().map_err(error)?
            {
                return Err(error(format!(
                    "baseline {baseline} already exists; compare with --baseline or save a new name"
                )));
            }
            pending.push(path);
        }
    }
    Ok(())
}

/// Criterion closure 无法返回 Result；明确保存失败并退出，避免输出虚假的成功统计。
fn required<T>(artifact: &Artifact, name: &str, result: Result<T>) -> T {
    match result {
        Ok(value) => value,
        Err(failure) => {
            if let Err(recording) = artifact.write_json(
                "status.json",
                &json!({"status":"failed", "scenario":name, "error":failure.to_string()}),
            ) {
                eprintln!("failed to record benchmark failure: {recording}");
            }
            eprintln!("benchmark {name} failed: {failure}");
            std::process::exit(1)
        }
    }
}

/// 固定次数的独立采样保留原先 timing 边界，Criterion 统计不复用这些观测。
fn sample<F>(
    harness: &mut Harness,
    name: &str,
    lifecycle: bool,
    mut setup: impl FnMut() -> Result<F>,
    mut operation: impl FnMut(&mut F, usize) -> Result,
    mut entities: impl FnMut(&mut F) -> Result<u32>,
) -> Result {
    let count = if lifecycle { 20 } else { SAMPLES };
    let mut fixture = setup()?;
    let mut durations = Vec::with_capacity(count);
    let start_entities = entities(&mut fixture)?;
    let mut end_entities = start_entities;
    let mut peak_entities = start_entities;
    if !lifecycle {
        for index in 0..WARMUP {
            operation(&mut fixture, index)?;
        }
    }
    for index in 0..count {
        if lifecycle && index > 0 {
            fixture = setup()?;
        }
        let start = Instant::now();
        operation(&mut fixture, index)?;
        durations.push(start.elapsed().as_secs_f64() * 1_000_000.0);
        end_entities = black_box(entities(&mut fixture)?);
        peak_entities = peak_entities.max(end_entities);
    }
    writeln!(
        harness.raw,
        "{}",
        json!({"scenario":name,"durations_us":durations})
    )
    .map_err(error)?;
    durations.sort_by(f64::total_cmp);
    let p95 = if lifecycle {
        String::new()
    } else {
        format!("{:.3}", durations[(count * 95).div_ceil(100) - 1])
    };
    writeln!(
        harness.csv,
        "{name},{count},{:.3},{p95},{:.3},{:.3},{start_entities},{end_entities},{peak_entities}",
        (durations[count / 2 - 1] + durations[count / 2]) / 2.0,
        durations[0],
        durations[count - 1],
    )
    .map_err(error)?;
    harness.csv.flush().map_err(error)?;
    harness.raw.flush().map_err(error)?;
    Ok(())
}

/// UI fixture 缺少必需的公开 Component 时明确失败，避免测量空路径。
pub fn missing(description: &str) -> BevyError {
    BevyError::error(format!("benchmark fixture missing {description}"))
}

/// 用真实 layout/text pipeline 创建独立 headless App，并在计时外加载内建字体。
/// 不创建 render device / native window，也不依赖未启用的系统字体发现。
pub fn ui_app() -> Result<App> {
    let mut app = crate::scene_app();
    crate::add_ui_plugins(&mut app);
    crate::spawn_ui_camera(&mut app, UVec2::new(1920, 1080), 1.0);
    app.add_plugins(WidgetryAssetPlugin);
    let font = app
        .world()
        .resource::<AssetServer>()
        .load::<Font>(BuiltinFont::Default.path());
    crate::advance_until(
        &mut app,
        Duration::from_secs(10),
        "benchmark 内建字体",
        |world| world.resource::<Assets<Font>>().contains(&font),
    )
    .map_err(BevyError::error)?;
    app.set_default_font(bevy::text::FontSource::Handle(font));
    Ok(app)
}

/// 三个真实 update 覆盖初始 layout、viewport projection 与其后 layout 消费；不替换 ComputedNode。
pub fn settle(app: &mut App) {
    for _ in 0..3 {
        app.update();
    }
}

/// 在计时外验证真实 Text 已完成 glyph layout，拒绝仅创建了 Text Component 的空 workload。
pub fn validate_text(app: &mut App) -> Result {
    let world = app.world_mut();
    let mut visible = 0;
    for (text, node, layout) in world
        .query::<(&Text, &ComputedNode, &TextLayoutInfo)>()
        .iter(world)
    {
        if !text.0.is_empty() && node.size().min_element() > 0.0 {
            visible += 1;
            if layout.glyphs.is_empty() {
                return Err(missing("Text glyph layout"));
            }
        }
    }
    if visible == 0 {
        return Err(missing("visible Text"));
    }
    Ok(())
}

impl Harness {
    /// Cargo bench 已在测量之前构建所有 target；运行时记录当前 executable 与源码。
    pub fn new(owner: &str) -> Result<Self> {
        let artifact = Artifact::new(owner, "bench; Cargo defaults opt-level=3")?;
        let report = artifact.workspace.join("target/criterion").join(owner);
        let arguments = std::env::args().skip(1).collect::<Vec<_>>();
        let observe = arguments.iter().any(|argument| argument == "--bench")
            && !arguments.iter().any(|argument| {
                argument == "--test"
                    || argument.starts_with("--profile-time")
                    || argument.starts_with("--load-baseline")
            });
        let saved = baseline_argument(&arguments, "--save-baseline", Some("-s"));
        for (flag, short) in [
            ("--save-baseline", Some("-s")),
            ("--baseline", Some("-b")),
            ("--baseline-lenient", None),
            ("--load-baseline", None),
        ] {
            if let Some(name) = baseline_argument(&arguments, flag, short) {
                validate_baseline_name(&name)?;
            }
        }
        let compare = baseline_argument(&arguments, "--baseline", Some("-b")).is_some()
            || baseline_argument(&arguments, "--baseline-lenient", None).is_some()
            || baseline_argument(&arguments, "--load-baseline", None).is_some()
            || arguments
                .iter()
                .any(|argument| argument == "--discard-baseline");
        let mut criterion = Criterion::default()
            .output_directory(&report)
            .sample_size(20)
            .warm_up_time(Duration::from_millis(100))
            .measurement_time(Duration::from_millis(500))
            .configure_from_args();
        if let Some(saved) = saved {
            ensure_new_baseline(&report, &saved)?;
            artifact.write_json(
                "criterion.json",
                &json!({"report":report,"saved_baseline":saved}),
            )?;
        } else if !compare {
            let name = artifact
                .directory
                .file_name()
                .ok_or_else(|| error("artifact directory missing name"))?
                .to_string_lossy()
                .into_owned();
            artifact.write_json(
                "criterion.json",
                &json!({"report":report,"saved_baseline":name}),
            )?;
            criterion = criterion.save_baseline(name);
        } else {
            artifact.write_json(
                "criterion.json",
                &json!({"report":report,"comparison_arguments":arguments}),
            )?;
        }
        let mut csv = File::create(artifact.directory.join("samples.csv")).map_err(error)?;
        writeln!(csv,"scenario,samples,median_us,p95_us,min_us,max_us,entities_start,entities_end,entities_peak").map_err(error)?;
        let raw = File::create(artifact.directory.join("latencies.jsonl")).map_err(error)?;
        artifact.write_json("status.json", &json!({"status":"running"}))?;
        Ok(Self {
            criterion,
            artifact,
            csv,
            raw,
            observe,
        })
    }

    /// 生成 Criterion HTML 汇总并明确标识完整成功。
    pub fn finish(self) -> Result {
        self.criterion.final_summary();
        self.artifact
            .write_json("status.json", &json!({"status":"complete"}))
    }
}

// 测试断言用于防止 benchmark 静默测量没有字体的空文本路径。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// 已有 named baseline 必须拒绝覆盖，不改变原始估计文件。
    #[test]
    fn baseline_is_never_overwritten() -> Result {
        let directory = std::env::temp_dir().join(format!(
            "widgetry-baseline-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(error)?
                .as_nanos()
        ));
        let baseline = directory.join("group/case/initial");
        fs::create_dir_all(&baseline).map_err(error)?;
        fs::write(baseline.join("estimates.json"), "original").map_err(error)?;
        let rejected = ensure_new_baseline(&directory, "initial").is_err();
        let preserved = fs::read_to_string(baseline.join("estimates.json")).map_err(error)?;
        let new_name = ensure_new_baseline(&directory, "next").is_ok();
        let rejected_case_variant = ensure_new_baseline(&directory, "INITIAL").is_err();
        fs::remove_dir_all(directory).map_err(error)?;
        assert!(rejected);
        assert_eq!(preserved, "original");
        assert!(new_name);
        assert!(rejected_case_variant);
        Ok(())
    }

    /// Criterion 内部输出目录不能成为 named baseline，Windows 大小写变体同样拒绝。
    #[test]
    fn baseline_rejects_internal_directories() -> Result {
        let directory = std::env::temp_dir().join(format!(
            "widgetry-reserved-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(error)?
                .as_nanos()
        ));
        for name in [
            "new", "NEW", "change", "CHANGE", "report", "REPORT", "profile", "PROFILE",
        ] {
            assert!(ensure_new_baseline(&directory, name).is_err(), "{name}");
        }
        Ok(())
    }

    /// baseline wrapper 保留 Criterion / Clap 合法的短参数 equals 与组合 flags 语义。
    #[test]
    fn baseline_preserves_short_option_syntax() {
        for (flag, short, argument) in [
            ("--save-baseline", "-s", "-s=trial"),
            ("--baseline", "-b", "-b=trial"),
            ("--save-baseline", "-s", "-vns=trial"),
            ("--baseline", "-b", "-vnbtrial"),
        ] {
            assert_eq!(
                baseline_argument(&[argument.into()], flag, Some(short)),
                Some("trial".into())
            );
        }
        assert_eq!(
            baseline_argument(
                &["-vns".into(), "trial".into()],
                "--save-baseline",
                Some("-s")
            ),
            Some("trial".into())
        );
        assert_eq!(
            baseline_argument(&["-btrial-s".into()], "--save-baseline", Some("-s")),
            None
        );
        assert_eq!(
            baseline_argument(
                &["--".into(), "-s=trial".into()],
                "--save-baseline",
                Some("-s")
            ),
            None
        );
    }

    /// benchmark fixture 在计时前具备可用字体，真实 Text 能完成 glyph 与 layout。
    #[test]
    fn fixture_renders_real_text() -> Result {
        let mut app = ui_app()?;
        app.world_mut()
            .spawn_scene(bsn! { Text("benchmark text") })?;
        settle(&mut app);
        assert!(validate_text(&mut app).is_ok());
        Ok(())
    }
}
