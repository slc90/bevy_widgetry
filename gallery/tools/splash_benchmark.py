"""运行优化 Gallery 的静态与动画路径，区分主线程 update、frame 和 present 排队间隔。"""
import argparse
import csv
import json
import os
from pathlib import Path
import platform
import subprocess
import sys


def summarize(path):
    with path.open(encoding="utf-8") as source:
        rows = [row for row in csv.DictReader(source) if float(row["elapsed_ms"]) >= 2000]
    summary = {"samples": len(rows)}
    for field in ("update_ms", "frame_interval_ms", "present_interval_ms"):
        values = sorted(float(row[field]) for row in rows if float(row[field]) > 0)
        if not values:
            raise RuntimeError(f"没有 {field} 样本")
        summary[field] = {
            "samples": len(values),
            "mean": sum(values) / len(values),
            "p95": values[int((len(values) - 1) * 0.95)],
            "max": values[-1],
            "over_16_7ms": sum(value > 16.7 for value in values),
            "over_25ms": sum(value > 25 for value in values),
        }
    with path.with_suffix(".gpu.csv").open(encoding="utf-8") as source:
        gpu = {}
        for row in csv.DictReader(source):
            if float(row["elapsed_ms"]) >= 2000:
                gpu.setdefault(row["path"], []).append(float(row["gpu_ms"]))
    summary["gpu_passes_ms"] = {}
    for name, values in gpu.items():
        values.sort()
        summary["gpu_passes_ms"][name] = {"samples": len(values), "mean": sum(values) / len(values), "p95": values[int((len(values) - 1) * 0.95)], "max": values[-1]}
    return summary


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--binary", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    if args.binary is None:
        subprocess.run(["cargo", "build", "--locked", "--release", "-p", "widget_gallery"], cwd=root, check=True)
    binary = (args.binary or root / "target/release/widget_gallery.exe").resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    report = {"platform": platform.platform(), "binary": str(binary), "profile": "release", "viewport": [1920, 1080], "warmup_ms": 2000,
              "timing": "update=First到Last；frame=Last间隔；present=DX12 present调用后的Render间隔，非GPU耗时或桌面显示latency",
              "git": subprocess.check_output(["git", "describe", "--always", "--dirty"], cwd=root, text=True).strip(),
              "rust": subprocess.check_output(["rustc", "--version"], text=True).strip()}
    report["hardware"] = json.loads(subprocess.check_output([
        "pwsh", "-NoProfile", "-Command",
        "@{cpu=(Get-CimInstance Win32_Processor).Name;gpu=(Get-CimInstance Win32_VideoController | Select-Object Name,CurrentRefreshRate,DriverVersion)} | ConvertTo-Json -Compress"
    ], text=True, encoding="utf-8"))
    for mode in ("baseline", "benchmark"):
        path = (args.output / f"{mode}.csv").resolve()
        env = dict(os.environ, GALLERY_SPLASH_PREVIEW=mode, GALLERY_SPLASH_BENCH_OUTPUT=str(path), BRP_EXTRAS_PORT="15989")
        with (args.output / f"{mode}.log").open("w", encoding="utf-8") as log:
            subprocess.run([str(binary)], cwd=root, env=env, stdout=log, stderr=log, check=True, timeout=90)
        report[mode] = summarize(path)
    (args.output / "summary.json").write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
