"""对照 Gallery、Splash 动画与淡出，记录 CPU、全部 Camera GPU、present 和清理。"""
import argparse
import csv
import json
import os
from pathlib import Path
import platform
import subprocess
import sys


def statistics(values):
    values = sorted(values)
    if not values:
        raise RuntimeError("没有计时样本")
    return {
        "samples": len(values), "mean": sum(values) / len(values),
        "p50": values[int((len(values) - 1) * 0.5)],
        "p95": values[int((len(values) - 1) * 0.95)], "max": values[-1],
        "over_16_7ms": sum(value > 16.7 for value in values),
        "over_25ms": sum(value > 25 for value in values),
    }


def summarize(path, mode):
    with path.open(encoding="utf-8") as source:
        all_rows = list(csv.DictReader(source))
    active = [row for row in all_rows if row["splash_active"] == "true"]
    steady_start = 2000.0
    if mode == "play":
        if not active:
            raise RuntimeError("完整播放未观察到 Splash")
        cleaned = [row for row in all_rows if float(row["startup_ms"]) > float(active[-1]["startup_ms"]) and row["splash_active"] == "false"]
        if not cleaned:
            raise RuntimeError("完整播放未清理 Splash")
        steady_start = float(cleaned[0]["elapsed_ms"]) + 2000.0
    rows = [row for row in all_rows if float(row["elapsed_ms"]) >= steady_start]
    summary = {"samples": len(rows), "steady_start_ms": steady_start}
    for field in ("update_ms", "frame_interval_ms", "present_interval_ms"):
        values = sorted(float(row[field]) for row in rows if float(row[field]) > 0)
        if not values:
            raise RuntimeError(f"没有 {field} 样本")
        summary[field] = statistics(values)
    with path.with_suffix(".gpu.csv").open(encoding="utf-8") as source:
        gpu = {}
        for row in csv.DictReader(source):
            if float(row["elapsed_ms"]) >= steady_start:
                gpu.setdefault(row["path"], []).append(float(row["gpu_ms"]))
    summary["gpu_passes_ms"] = {}
    for name, values in gpu.items():
        summary["gpu_passes_ms"][name] = statistics(values)
    total = summary["gpu_passes_ms"].get("render/splash_total/elapsed_gpu")
    if total is None:
        raise RuntimeError("缺少覆盖全部 Camera 的 GPU timestamp 样本")
    presented = [row for row in all_rows if row["presented"] == "true"]
    if not presented:
        raise RuntimeError("未观察到首帧 present")
    summary["first_present_observed_ms"] = float(presented[0]["startup_ms"])
    for field in ("cpu_assets_ready", "source_ready"):
        ready = next((row for row in all_rows if row[field] == "true"), None)
        summary[f"{field}_observed_ms"] = float(ready["startup_ms"]) if ready else None
    if mode == "play":
        summary["cleanup_observed_ms"] = float(cleaned[0]["startup_ms"])
        playing = [row for row in active if row["presented"] == "true" and float(row["alpha"]) >= 1.0]
        summary["frame_indices"] = sorted({int(row["frame_index"]) for row in playing})
        summary["animation_elapsed_ms"] = max(float(row["animation_ms"]) for row in active)
        fading = [row for row in active if 0.0 < float(row["alpha"]) < 1.0]
        summary["fade_samples"] = len(fading)
        summary["fade_frame_interval_ms"] = statistics(float(row["frame_interval_ms"]) for row in fading)

        summary["startup_update_ms"] = statistics(float(row["update_ms"]) for row in active if row["presented"] == "false")
        summary["play_update_ms"] = statistics(float(row["update_ms"]) for row in active if row["presented"] == "true")
        summary["play_frame_interval_ms"] = statistics(float(row["frame_interval_ms"]) for row in presented if row["splash_active"] == "true" and float(row["frame_interval_ms"]) > 0)
        summary["cleanup_counts"] = {field: [int(cleaned[0][field]), int(cleaned[-1][field])] for field in ("entities", "images")}
    return summary


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--modes", nargs="+", choices=("gallery", "static", "benchmark", "play"), default=("gallery", "static", "benchmark", "play"))
    parser.add_argument("--repeat", type=int, default=2)
    args = parser.parse_args()
    if args.repeat < 1:
        parser.error("--repeat 必须大于 0")
    root = Path(__file__).resolve().parents[2]
    if args.binary is None:
        subprocess.run(["cargo", "build", "--locked", "--release", "-p", "widget_gallery"], cwd=root, check=True)
    binary = (args.binary or root / "target/release/widget_gallery.exe").resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {"platform": platform.platform(), "binary": str(binary), "profile": "release", "viewport": [1920, 1080], "warmup_ms": 2000,
              "timing": "update=First到Last；frame=Last间隔；present=主窗口DX12 present后的Render间隔，非桌面显示latency；render/splash_total=全部Camera提交的GPU timestamp；其他同名GPU路径仅末次Camera样本；startup=benchmark安装后（DX12初始化后）至主线程观察present，不是进程冷启动",
              "instrumentation": "仅测量模式持续RequestRedraw，保留desktop_app；未清除OS或Shader cache；play稳态从实际cleanup再预热2秒后起算，其他模式从首present预热2秒起算；另列完整演出帧；日志/CSV/GPU诊断带来测量开销",
              "git": subprocess.check_output(["git", "describe", "--always", "--dirty"], cwd=root, text=True).strip(),
              "rust": subprocess.check_output(["rustc", "--version"], text=True).strip()}
    report["hardware"] = json.loads(subprocess.check_output([
        "pwsh", "-NoProfile", "-Command",
        "@{cpu=(Get-CimInstance Win32_Processor).Name;gpu=(Get-CimInstance Win32_VideoController | Select-Object Name,CurrentRefreshRate,DriverVersion)} | ConvertTo-Json -Compress"
    ], text=True, encoding="utf-8"))
    report["runs"] = {}
    for repeat in range(args.repeat):
        for mode in args.modes:
            name = f"{mode}-{repeat + 1}"
            path = (args.output / f"{name}.csv").resolve()
            env = dict(os.environ)
            for key in tuple(env):
                if key.startswith("GALLERY_SPLASH_") or key in ("GALLERY_WAVEFORM_BENCH_OUTPUT", "GALLERY_FILE_DIALOG_BENCH_OUTPUT", "GALLERY_PAGE_LIFECYCLE_BENCH_OUTPUT"):
                    del env[key]
            env.update(GALLERY_SPLASH_PREVIEW=mode, GALLERY_SPLASH_BENCH_OUTPUT=str(path), BRP_EXTRAS_PORT="15989")
            if mode == "gallery":
                env["GALLERY_SPLASH_DISABLE"] = "1"
            with (args.output / f"{name}.log").open("w", encoding="utf-8") as log:
                subprocess.run([str(binary)], cwd=root, env=env, stdout=log, stderr=log, check=True, timeout=90)
            log_text = (args.output / f"{name}.log").read_text(encoding="utf-8")
            if "panicked at" in log_text or " ERROR " in log_text:
                raise RuntimeError(f"{name} 运行日志包含错误，检查对应 log")
            report["runs"][name] = summarize(path, mode)
            print(f"完成 {name}", flush=True)
    if "gallery" in args.modes:
        report["gpu_extra_ms"] = {}
        for repeat in range(args.repeat):
            baseline = report["runs"][f"gallery-{repeat + 1}"]["gpu_passes_ms"]["render/splash_total/elapsed_gpu"]["mean"]
            for mode in args.modes:
                value = report["runs"][f"{mode}-{repeat + 1}"]["gpu_passes_ms"]["render/splash_total/elapsed_gpu"]["mean"]
                report["gpu_extra_ms"][f"{mode}-{repeat + 1}"] = value - baseline
    (args.output / "summary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
