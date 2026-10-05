"""Collect real BRP mouse paths and analyze App-side FileDialog trace events.

Render submission is reported separately. Display latency requires a correlated
presentation adapter using the trace's clock_id and app_monotonic_ns clock.
"""
import argparse
import json
from pathlib import Path
import platform
import statistics
import time
import urllib.request


def rpc(port, method, params):
    payload = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    request = urllib.request.Request(f"http://127.0.0.1:{port}", data=payload,
                                     headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=30) as response:
        value = json.load(response)
    if "error" in value:
        raise RuntimeError(value["error"])
    return value["result"]


def events(path):
    lines = path.read_text(encoding="utf-8").splitlines(keepends=True)
    return [json.loads(line) for line in lines if line.endswith("\n")]


def named_geometry(port, name):
    rows = rpc(port, "world.query", {"data": {"components": ["bevy_ecs::name::Name"],
                "option": ["bevy_ui::ui_transform::UiGlobalTransform", "bevy_ui::ui_node::ComputedNode"]},
                "filter": {"with": ["bevy_ecs::name::Name"]}})
    rows = [row for row in rows if row["components"]["bevy_ecs::name::Name"] == name]
    if not rows:
        raise ValueError(f"Named UI missing: {name}")
    return rows


def point(row):
    transform = row["components"]["bevy_ui::ui_transform::UiGlobalTransform"]
    size = row["components"]["bevy_ui::ui_node::ComputedNode"]["size"]
    if min(size) <= 0:
        raise ValueError("UI target is not laid out")
    scale = row["components"]["bevy_ui::ui_node::ComputedNode"]["inverse_scale_factor"]
    return [value * scale for value in transform[-2:]]


def click_named(port, name, window):
    rows = named_geometry(port, name)
    if len(rows) != 1:
        raise ValueError(f"Expected unique named UI: {name}")
    rpc(port, "brp_extras/move_mouse", {"position": point(rows[0]), "window": window})
    time.sleep(0.15)
    rpc(port, "brp_extras/click_mouse", {"button": "Left", "window": window})
    time.sleep(0.1)


def collect(args):
    trace = []
    cursor = 0
    resources = []

    def read_trace():
        nonlocal cursor
        response = rpc(args.port, "benchmark/file_dialog_trace", {"from": cursor})
        cursor = response["next"]
        trace.extend(response["events"])
        resources.append(response["resources"])
        return trace

    def directory_ready(root, predicate):
        deadline = time.monotonic() + args.timeout
        while time.monotonic() < deadline:
            read_trace()
            dialogs = [dialog for dialog in resources[-1]["dialogs"] if dialog["root"] == root]
            if len(dialogs) == 1:
                dialog = dialogs[0]
                if dialog["directory_state"].startswith("Failed"):
                    raise RuntimeError("Navigation directory failed")
                if predicate(dialog["path"]) and dialog["directory_state"] == "Ready" and not dialog["projection_pending"]:
                    return dialog
            time.sleep(0.05)
        raise TimeoutError("Navigation did not reach the expected Ready directory")

    windows = rpc(args.port, "world.query", {"data": {}, "filter": {
        "with": ["bevy_window::window::PrimaryWindow", "bevy_window::window::Window"]}})
    if len(windows) != 1:
        raise RuntimeError("Expected exactly one PrimaryWindow for launcher input")
    primary = windows[0]["entity"]
    for cycle in range(args.samples):
        previous = {event["sample"] for event in read_trace() if "sample" in event}
        launcher_position = args.launcher_position
        if launcher_name := getattr(args, "launcher_name", None):
            launcher_position = point(named_geometry(args.port, launcher_name)[0])
        # 未指定 window 时 BRP 会复用上次 cursor window，Cancel 后该 window 已失效。
        rpc(args.port, "brp_extras/move_mouse", {"position": launcher_position, "window": primary})
        # BRP 输入按 update 入队。先让 pointer hover 收敛，避免同帧 Click 命中旧位置。
        time.sleep(0.15)
        rpc(args.port, "brp_extras/click_mouse", {"button": "Left", "window": primary})
        # Do not query the App during its 150ms first-display budget.
        time.sleep(max(0.35, getattr(args, "quiet_wait_ms", 0) / 1000))
        deadline = time.monotonic() + args.timeout
        sample = None
        while time.monotonic() < deadline:
            trace = read_trace()
            started = [event for event in trace if event.get("event") == "activate"
                       and event["sample"] not in previous]
            if len(started) > 1:
                raise RuntimeError("Concurrent launcher activations invalidate sequential collection")
            if started:
                sample = started[0]["sample"]
                if started[0]["operation"] != args.operation or started[0]["t_input"] is None:
                    raise RuntimeError("Launcher or App input boundary did not match the sample")
                expected_modal = getattr(args, "expected_modal", None)
                if expected_modal is not None and started[0].get("modal") != expected_modal:
                    raise RuntimeError("Actual launcher modality does not match this scenario")
                points = {event["event"]: event for event in trace if event.get("sample") == sample}
                if getattr(args, "control", False) and "scene" in points:
                    windows = rpc(args.port, "world.query", {"data": {}, "filter": {"with": ["bevy_window::window::Window"], "without": ["bevy_window::window::PrimaryWindow"]}})
                    if len(windows) == 1:
                        content = {"root":points["scene"]["root"], "native_window":windows[0]["entity"]}
                        break
                if "first_content_frame" in points and (getattr(args, "cancel_loading", False) or "final_projection" in points):
                    content = points["first_content_frame"]
                    break
            time.sleep(0.05)
        else:
            rpc(args.port, "brp_extras/screenshot", {"path": str(args.events.parent / "failed-primary.png")})
            args.events.with_suffix(".failed.json").write_text(json.dumps({
                "cycle": cycle + 1, "sample": sample, "launcher_position": launcher_position,
                "resources": resources[-1]}, indent=2), encoding="utf-8")
            raise TimeoutError(f"Missing content/projection evidence for sample {sample}")
        window = content["native_window"]
        if getattr(args, "navigate", False):
            original = directory_ready(content["root"], lambda path: path is not None)["path"]
            row = min(named_geometry(args.port, "FileDialogEntryRow"), key=lambda row: point(row)[1])
            rpc(args.port, "brp_extras/move_mouse", {"position": point(row), "window": window})
            time.sleep(0.15)
            rpc(args.port, "brp_extras/double_click_mouse", {"button": "Left", "window": window})
            directory_ready(content["root"], lambda path: path is not None and path != original)
            click_named(args.port, "FileDialogBack", window)
            directory_ready(content["root"], lambda path: path == original)
        for index in range(getattr(args, "scrolls", 0)):
            area = named_geometry(args.port, "FileDialogEntries")[0]
            rpc(args.port, "brp_extras/move_mouse", {"position": point(area), "window": window})
            time.sleep(0.05)
            rpc(args.port, "brp_extras/scroll_mouse", {"x": 0, "y": -3 if index % 2 == 0 else 3, "unit": "Line", "window": window})
            time.sleep(0.05)
        if getattr(args, "screenshot", False) and not previous:
            rpc(args.port, "brp_extras/screenshot", {"camera": content["camera"], "path": str(args.events.parent / "shell.png")})
        rpc(args.port, "brp_extras/move_mouse", {"position": args.cancel_position, "window": window})
        time.sleep(0.15)
        rpc(args.port, "brp_extras/click_mouse", {"button": "Left", "window": window})
        # Wait for actual owned cleanup rather than mutating ECS to reset the fixture.
        close_deadline = time.monotonic() + args.timeout
        while time.monotonic() < close_deadline:
            windows = rpc(args.port, "world.query", {"data": {}, "filter": {
                "with": ["bevy_window::window::Window"]}})
            if not any(row["entity"] == window for row in windows):
                break
            time.sleep(0.05)
        else:
            raise TimeoutError("Cancel did not reclaim the native Window")
        read_trace()
        if (cycle + 1) % 20 == 0 and (observer := getattr(args, "thread_observer", None)):
            resources[-1]["process"] = observer()
            resources[-1]["cycle"] = cycle + 1
        print(f"Collected sample {sample}", flush=True)
    read_trace()
    rpc(args.port, "benchmark/file_dialog_export", {})
    args.events.with_suffix(".resources.json").write_text(json.dumps(resources, indent=2), encoding="utf-8")


def analyze(args):
    trace = events(args.events)
    protocols = [event for event in trace if event.get("event") == "protocol"]
    if len(protocols) != 1 or protocols[0].get("protocol") not in [1, 2]:
        raise ValueError("Expected one FileDialog protocol-v1/v2 trace")
    protocol = protocols[0]
    samples = {}
    for event in trace:
        if "sample" not in event:
            continue
        points = samples.setdefault(event["sample"], {})
        if event["event"] in points:
            raise ValueError(f"Duplicate {event['event']} for sample {event['sample']}")
        points[event["event"]] = event
    presented = {}
    if args.presentation:
        for event in events(args.presentation):
            if event.get("clock_id") != protocol["clock_id"] or event.get("endpoint") != "displayed":
                raise ValueError("Presentation evidence must use the same App clock and displayed endpoint")
            if not event.get("source") or not event.get("artifact"):
                raise ValueError("Presentation adapter must identify its source and evidence artifact")
            artifact = Path(event["artifact"])
            if not artifact.is_absolute():
                artifact = args.presentation.parent / artifact
            if not artifact.is_file():
                raise ValueError(f"Presentation evidence artifact missing: {artifact}")
            if event["sample"] in presented:
                raise ValueError("Duplicate presentation record")
            presented[event["sample"]] = event
    report = {"protocol": protocol, "collector_os": platform.platform(), "samples": [],
              "boundary": "App raw-input reader to correlated render submission; display needs adapter",
              "brp_round_trip_in_metric": False}
    for sample_id, points in sorted(samples.items()):
        row = {"sample": sample_id, "valid_submission": False, "display_verified": False}
        report["samples"].append(row)
        required = {"activate", "scene", "camera_ready", "cpu_content_ready", "first_content_frame"}
        if missing := required - points.keys():
            row["invalid_reason"] = f"Missing points: {sorted(missing)}"
            continue
        activation, scene, content = points["activate"], points["scene"], points["first_content_frame"]
        if "final_projection" in points and points["final_projection"]["data"]["state"] != "Ready":
            row["invalid_reason"] = "Directory did not reach Ready"
            continue
        row.update(operation=activation["operation"], root=scene["root"], session=scene["session"],
                   modal=activation.get("modal"), physical_size=content.get("physical_size"), scale_factor=content.get("scale_factor"),
                   native_window=content["native_window"], camera=content["camera"], app_frame=content["app_frame"])
        start = activation["t_input"]
        if start is None or not start <= activation["t_activate"] <= scene["t_scene"] <= content["ns"]:
            row["invalid_reason"] = "Input/timestamp order mismatch"
            continue
        for event in [points[name] for name in required - {"activate", "scene"}]:
            if event["root"] != scene["root"] or event["session"] != scene["session"]:
                raise ValueError("Root/session correlation mismatch")
        for event in [points["camera_ready"], points["cpu_content_ready"]]:
            if event["native_window"] != content["native_window"] or event["camera"] != content["camera"] or event["ns"] > content["ns"]:
                raise ValueError("Camera/content correlation mismatch")
        row["valid_submission"] = True
        row["input_to_submission_ms"] = (content["ns"] - start) / 1e6
        row["activate_to_submission_ms"] = (content["ns"] - activation["t_activate"]) / 1e6
        for name in ["camera_ready", "cpu_content_ready", "first_batch", "final_projection"]:
            row[f"input_to_{name}_ms"] = (points[name]["ns"] - start) / 1e6 if name in points else None
        if sample_id in presented:
            display = presented[sample_id]
            if any(display.get(key) != content[key] for key in ["root", "session", "native_window", "camera", "app_frame"]):
                raise ValueError("Presentation must match the exact content frame")
            # A blocking presentation call can return after its displayed-frame
            # timestamp. Correlation uses the exact frame, not post-call ordering.
            if display["ns"] < points["cpu_content_ready"]["ns"]:
                raise ValueError("Display timestamp precedes this content becoming ready")
            row["display_verified"] = True
            row["input_to_presented_ms"] = (display["ns"] - start) / 1e6
            row["display_uncertainty_ms"] = display.get("uncertainty_ns", 0) / 1e6
            row["input_to_presented_upper_ms"] = row["input_to_presented_ms"] + row["display_uncertainty_ms"]
            row["activate_to_presented_ms"] = (display["ns"] - activation["t_activate"]) / 1e6
    valid = [row for row in report["samples"] if row["valid_submission"]]
    display = [row for row in valid if row["display_verified"]]
    report["display_status"] = "verified" if display and len(display) == len(report["samples"]) else "incomplete"
    report["150ms_target_passed"] = all(row["input_to_presented_upper_ms"] <= 150 for row in display) if report["display_status"] == "verified" else None
    report["invalid_submission_count"] = len(samples) - len(valid)
    report["directory_projection_complete_count"] = sum("final_projection" in points for points in samples.values())
    report["unverified_display_count"] = len(samples) - len(display)
    report["statistics"] = {}
    report["frame_statistics"] = {}
    for stage in ["main_frame", "render_frame", "render_schedule", "window_acquire", "probe_cpu_frame"]:
        values = sorted(event["duration_ns"] / 1e6 for event in trace if event.get("event") == stage)
        if values:
            report["frame_statistics"][stage] = {"n":len(values), "median_ms":statistics.median(values),
                "p95_ms":values[max(0, (95 * len(values) + 99) // 100 - 1)], "max_ms":max(values),
                "over_16_7ms":sum(value > 16.7 for value in values), "over_2ms":sum(value > 2 for value in values)}
    report["steady_frame_statistics"] = {}
    opening = [(points["activate"]["app_frame"], points["first_content_frame"]["app_frame"])
               for points in samples.values() if "activate" in points and "first_content_frame" in points]
    for stage in ["main_frame", "render_schedule", "window_acquire"]:
        values = sorted(event["duration_ns"] / 1e6 for event in trace if event.get("event") == stage
                        and event.get("dialog_active") and not any(start <= event["app_frame"] <= end for start, end in opening))
        if values:
            report["steady_frame_statistics"][stage] = {"n":len(values), "median_ms":statistics.median(values),
                "p95_ms":values[max(0, (95 * len(values) + 99) // 100 - 1)], "max_ms":max(values),
                "over_16_7ms":sum(value > 16.7 for value in values)}
    for metric in ["input_to_submission_ms", "input_to_presented_ms", "input_to_first_batch_ms", "input_to_final_projection_ms"]:
        values = sorted(row[metric] for row in valid if row.get(metric) is not None)
        if values:
            report["statistics"][metric] = {"n":len(values), "median":statistics.median(values),
                                             "p95":values[max(0, (95 * len(values) + 99) // 100 - 1)], "max":max(values)}
    if valid:
        first = report["samples"][0]
        report["first_submission_ms"] = first.get("input_to_submission_ms")
        later = [row["input_to_submission_ms"] for row in report["samples"][1:] if row["valid_submission"]]
        report["subsequent_submission_median_ms"] = statistics.median(later) if later else None
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({key: value for key, value in report.items() if key != "samples"}, ensure_ascii=False, indent=2))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", type=Path, help="events.jsonl from the running Gallery")
    parser.add_argument("--output", type=Path, default=Path("report.json"))
    parser.add_argument("--presentation", type=Path, help="Same-clock displayed-frame evidence from a presentation adapter")
    parser.add_argument("--collect", action="store_true", help="Trigger sequential real mouse opens and Cancel")
    parser.add_argument("--navigate", action="store_true", help="Enter the first fixture folder and return with Back")
    parser.add_argument("--scrolls", type=int, default=0)
    parser.add_argument("--cancel-loading", action="store_true", help="Cancel after the shell while controlled I/O is still blocked")
    parser.add_argument("--screenshot", action="store_true", help="Capture first sample content after its latency boundary")
    parser.add_argument("--port", type=int, default=15702)
    parser.add_argument("--operation", default="OpenFile")
    parser.add_argument("--launcher-position", nargs=2, type=float)
    parser.add_argument("--cancel-position", nargs=2, type=float, default=[959, 673])
    parser.add_argument("--samples", type=int, default=20)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--fixture", type=Path, help="Create a new deterministic directory and exit")
    parser.add_argument("--entries", type=int, default=1000)
    args = parser.parse_args()
    if args.fixture:
        if args.entries < 0:
            parser.error("--entries must be nonnegative")
        args.fixture.mkdir(parents=True, exist_ok=False)
        for index in range(args.entries):
            if index % 20 == 0:
                (args.fixture / f"folder_{index:06}").mkdir()
            else:
                (args.fixture / f"file_{index:06}.{'png' if index % 3 == 0 else 'txt'}").write_bytes(b"fixture\n")
        return
    if not args.events or (args.collect and not args.launcher_position) or args.samples < 1:
        parser.error("--events is required; --collect also requires --launcher-position and positive --samples")
    if args.collect:
        try:
            collect(args)
        finally:
            rpc(args.port, "benchmark/file_dialog_export", {})
    analyze(args)


if __name__ == "__main__":
    main()
