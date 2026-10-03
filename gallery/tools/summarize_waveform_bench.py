import csv
import json
import math
import statistics
import sys
from pathlib import Path


def distribution(values):
    values = sorted(values)
    if not values:
        return None
    return {
        "samples": len(values),
        "median": statistics.median(values),
        "p95": values[math.ceil(len(values) * 0.95) - 1] if len(values) >= 100 else None,
        "p99": values[math.ceil(len(values) * 0.99) - 1] if len(values) >= 1000 else None,
        "max": values[-1],
    }


def main():
    directory = Path(sys.argv[1])
    if (directory / "run").is_dir():
        directory = directory / "run"
    with (directory / "frames.csv").open(encoding="utf-8") as file:
        frames = list(csv.DictReader(file))
    steady = [frame for frame in frames if float(frame["elapsed_s"]) > 12 and frame["capture"] == "false" and frame["burst"] == "false"]
    if len(steady) < 1000:
        raise ValueError("稳态样本不足，不能形成 P99 baseline")
    result = {key: distribution(float(frame[key]) for frame in steady) for key in ["interval_ms", "main_update_ms", "producer_ms"]}
    for key in ["asset_extract_ms", "render_schedule_ms"]:
        if key in frames[0]:
            result[key] = distribution(float(frame[key]) for frame in steady)
    result["fps_steady"] = 1000 / statistics.mean(float(frame["interval_ms"]) for frame in steady)
    result["fps_all"] = (len(frames) - 1) / (float(frames[-1]["elapsed_s"]) - float(frames[0]["elapsed_s"]))
    result["throughput_frames_per_second"] = (int(frames[-1]["producer_frame"]) - int(frames[0]["producer_frame"])) / (float(frames[-1]["elapsed_s"]) - float(frames[0]["elapsed_s"]))
    result["maximum_backlog"] = max(int(frame["backlog"]) for frame in frames)
    result["dropped_samples"] = max(int(frame["dropped_samples"]) for frame in frames)
    burst = [frame for frame in frames if frame["burst"] == "true"]
    result["burst"] = {"maximum_read_frames": max(int(frame["read_frames"]) for frame in burst), "main_update_ms": distribution(float(frame["main_update_ms"]) for frame in burst)}
    result["working_sets"] = {key: {"min": min(int(frame[key]) for frame in steady), "max": max(int(frame[key]) for frame in steady)} for key in [key for key in ["raw_bytes", "reduced_bytes", "mesh_bytes", "entities", "entity_indices", "active_entities", "images", "meshes"] if key in frames[0]]}
    with (directory / "observable.csv").open(encoding="utf-8") as file:
        captures = list(csv.DictReader(file))
    result["readback_upper_ms"] = distribution(float(capture["readback_upper_ms"]) for capture in captures)
    result["all_captures_have_64_lanes"] = bool(captures) and all(capture["pixels_nonempty"] == "true" for capture in captures)
    diagnostics = {}
    with (directory / "render.csv").open(encoding="utf-8") as file:
        for row in csv.DictReader(file):
            if int(row["frame"]) > 720:
                diagnostics.setdefault(row["path"], []).append(float(row["value"]))
    result["render_diagnostics"] = {name: distribution(values) for name, values in diagnostics.items() if "elapsed_" in name or "allocator" in name}
    with (directory / "summary.json").open("x", encoding="utf-8") as file:
        json.dump(result, file, indent=2)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
