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


RPC_TIMEOUT_SECONDS = 30
HOVER_SETTLE_SECONDS = 0.15
CLICK_SETTLE_SECONDS = 0.1
INVALID_PARAMS_CODE = -32602
POLL_INTERVAL_SECONDS = 0.05
SCROLL_SETTLE_SECONDS = 0.05
MIN_QUIET_SECONDS = 0.35
MILLISECONDS_PER_SECOND = 1000
SCROLL_LINES = 3
OBSERVER_SAMPLE_INTERVAL = 20
NANOS_PER_MILLISECOND = 1e6
FIRST_DISPLAY_TARGET_MS = 150
P95_PERCENTILE = 95
PERCENTILE_CEIL_OFFSET = 99
PERCENT_SCALE = 100
FRAME_BUDGET_MS = 16.7
UPDATE_BUDGET_MS = 2
DEFAULT_BRP_PORT = 15702
DEFAULT_SAMPLES = 20
DEFAULT_TIMEOUT_SECONDS = 30
DEFAULT_ENTRIES = 1000

def rpc(port, method, params):
    payload = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    request = urllib.request.Request(f"http://127.0.0.1:{port}", data=payload,
                                     headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=RPC_TIMEOUT_SECONDS) as response:
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
    time.sleep(HOVER_SETTLE_SECONDS)
    rpc(port, "brp_extras/click_mouse", {"button": "Left", "window": window})
    time.sleep(CLICK_SETTLE_SECONDS)


def finish_pointer(args, closed):
    rpc(args.port, "brp_extras/pointer_control", {"action": "release"})
    deadline = time.monotonic() + args.timeout
    while True:
        status = rpc(args.port, "brp_extras/pointer_control", {"action": "status"})
        if status["phase"] == "inactive":
            error = status["last_error"]
            expected = closed and error == {
                "window": closed["window"], "generation": closed["generation"],
                "method": "brp_extras/click_mouse", "code": INVALID_PARAMS_CODE,
                "message": "Target window was destroyed"}
            if error is not None and not expected:
                raise RuntimeError(f"Pointer input failed: {error}")
            return status
        if time.monotonic() >= deadline:
            raise TimeoutError(f"Pointer cleanup did not finish: {status}")
        time.sleep(POLL_INTERVAL_SECONDS)


def collect(args):
    closed = {}
    try:
        return collect_samples(args, closed)
    finally:
        finish_pointer(args, closed)


def collect_samples(args, closed):
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
            time.sleep(POLL_INTERVAL_SECONDS)
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
        # BRP reuses the previous cursor window when none is specified, but Cancel has already invalidated it.
        rpc(args.port, "brp_extras/move_mouse", {"position": launcher_position, "window": primary})
        closed.clear()
        # BRP queues input per update.
        # Let Pointer hover settle first so a same-frame Click cannot hit the old position.
        time.sleep(HOVER_SETTLE_SECONDS)
        rpc(args.port, "brp_extras/click_mouse", {"button": "Left", "window": primary})
        # Do not query the App during its 150ms first-display budget.
        time.sleep(max(MIN_QUIET_SECONDS, getattr(args, "quiet_wait_ms", 0) / MILLISECONDS_PER_SECOND))
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
            time.sleep(POLL_INTERVAL_SECONDS)
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
            time.sleep(HOVER_SETTLE_SECONDS)
            rpc(args.port, "brp_extras/double_click_mouse", {"button": "Left", "window": window})
            directory_ready(content["root"], lambda path: path is not None and path != original)
            click_named(args.port, "FileDialogBack", window)
            directory_ready(content["root"], lambda path: path == original)
        for index in range(getattr(args, "scrolls", 0)):
            area = named_geometry(args.port, "FileDialogEntries")[0]
            rpc(args.port, "brp_extras/move_mouse", {"position": point(area), "window": window})
            time.sleep(SCROLL_SETTLE_SECONDS)
            rpc(args.port, "brp_extras/scroll_mouse", {"x": 0, "y": -SCROLL_LINES if index % 2 == 0 else SCROLL_LINES, "unit": "Line", "window": window})
            time.sleep(SCROLL_SETTLE_SECONDS)
        if getattr(args, "screenshot", False) and not previous:
            rpc(args.port, "brp_extras/screenshot", {"camera": content["camera"], "path": str(args.events.parent / "shell.png")})
        cancel_position = args.cancel_position
        if cancel_position is None:
            buttons = named_geometry(args.port, "FileDialogCancel")
            if len(buttons) != 1:
                raise ValueError("Expected one FileDialogCancel control")
            cancel_position = point(buttons[0])
        rpc(args.port, "brp_extras/move_mouse", {"position": cancel_position, "window": window})
        time.sleep(HOVER_SETTLE_SECONDS)
        status = rpc(args.port, "brp_extras/pointer_control", {"action": "status"})
        if status["last_error"] is not None or status["window"] != window or status["busy"]:
            raise RuntimeError(f"Pointer was not ready for Cancel: {status}")
        rpc(args.port, "brp_extras/click_mouse", {"button": "Left", "window": window})
        # Wait for actual owned cleanup rather than mutating ECS to reset the fixture.
        close_deadline = time.monotonic() + args.timeout
        while time.monotonic() < close_deadline:
            windows = rpc(args.port, "world.query", {"data": {}, "filter": {
                "with": ["bevy_window::window::Window"]}})
            if not any(row["entity"] == window for row in windows):
                break
            time.sleep(POLL_INTERVAL_SECONDS)
        else:
            raise TimeoutError("Cancel did not reclaim the native Window")
        closed.update(window=window, generation=status["generation"])
        pointer_cleanup = finish_pointer(args, closed)
        read_trace()
        resources[-1]["pointer_cleanup"] = pointer_cleanup
        if (cycle + 1) % OBSERVER_SAMPLE_INTERVAL == 0 and (observer := getattr(args, "thread_observer", None)):
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
              "boundary": "App Pointer Left Release / keyboard reader after Picking input to correlated render submission; display needs adapter",
              "brp_round_trip_in_metric": False}
    for sample_id, points in sorted(samples.items()):
        row = {"sample": sample_id, "valid_submission": False, "display_verified": False,
               "150ms_target_passed": None}
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
        row["input_to_submission_ms"] = (content["ns"] - start) / NANOS_PER_MILLISECOND
        row["activate_to_submission_ms"] = (content["ns"] - activation["t_activate"]) / NANOS_PER_MILLISECOND
        for name in ["camera_ready", "cpu_content_ready", "first_batch", "final_projection"]:
            row[f"input_to_{name}_ms"] = (points[name]["ns"] - start) / NANOS_PER_MILLISECOND if name in points else None
        if sample_id in presented:
            display = presented[sample_id]
            if any(display.get(key) != content[key] for key in ["root", "session", "native_window", "camera", "app_frame"]):
                raise ValueError("Presentation must match the exact content frame")
            # A blocking presentation call can return after its displayed-frame
            # timestamp. Correlation uses the exact frame, not post-call ordering.
            if display["ns"] < points["cpu_content_ready"]["ns"]:
                raise ValueError("Display timestamp precedes this content becoming ready")
            row["display_verified"] = True
            row["input_to_presented_ms"] = (display["ns"] - start) / NANOS_PER_MILLISECOND
            row["display_uncertainty_ms"] = display.get("uncertainty_ns", 0) / NANOS_PER_MILLISECOND
            row["input_to_presented_upper_ms"] = row["input_to_presented_ms"] + row["display_uncertainty_ms"]
            row["150ms_target_passed"] = row["input_to_presented_upper_ms"] <= FIRST_DISPLAY_TARGET_MS
            row["activate_to_presented_ms"] = (display["ns"] - activation["t_activate"]) / NANOS_PER_MILLISECOND
    valid = [row for row in report["samples"] if row["valid_submission"]]
    display = [row for row in valid if row["display_verified"]]
    report["display_status"] = "verified" if display and len(display) == len(report["samples"]) else "incomplete"
    report["150ms_target_passed"] = all(row["input_to_presented_upper_ms"] <= FIRST_DISPLAY_TARGET_MS for row in display) if report["display_status"] == "verified" else None
    report["invalid_submission_count"] = len(samples) - len(valid)
    report["directory_projection_complete_count"] = sum("final_projection" in points for points in samples.values())
    report["unverified_display_count"] = len(samples) - len(display)
    report["statistics"] = {}
    report["frame_statistics"] = {}
    for stage in ["main_frame", "render_frame", "render_schedule", "window_acquire", "probe_cpu_frame"]:
        values = sorted(event["duration_ns"] / NANOS_PER_MILLISECOND for event in trace if event.get("event") == stage)
        if values:
            report["frame_statistics"][stage] = {"n":len(values), "median_ms":statistics.median(values),
                "p95_ms":values[max(0, (P95_PERCENTILE * len(values) + PERCENTILE_CEIL_OFFSET) // PERCENT_SCALE - 1)], "max_ms":max(values),
                "over_16_7ms":sum(value > FRAME_BUDGET_MS for value in values), "over_2ms":sum(value > UPDATE_BUDGET_MS for value in values)}
    report["steady_frame_statistics"] = {}
    opening = [(points["activate"]["app_frame"], points["first_content_frame"]["app_frame"])
               for points in samples.values() if "activate" in points and "first_content_frame" in points]
    for stage in ["main_frame", "render_schedule", "window_acquire"]:
        values = sorted(event["duration_ns"] / NANOS_PER_MILLISECOND for event in trace if event.get("event") == stage
                        and event.get("dialog_active") and not any(start <= event["app_frame"] <= end for start, end in opening))
        if values:
            report["steady_frame_statistics"][stage] = {"n":len(values), "median_ms":statistics.median(values),
                "p95_ms":values[max(0, (P95_PERCENTILE * len(values) + PERCENTILE_CEIL_OFFSET) // PERCENT_SCALE - 1)], "max_ms":max(values),
                "over_16_7ms":sum(value > FRAME_BUDGET_MS for value in values)}
    for metric in ["input_to_submission_ms", "input_to_presented_ms", "input_to_first_batch_ms", "input_to_final_projection_ms"]:
        values = sorted(row[metric] for row in valid if row.get(metric) is not None)
        if values:
            report["statistics"][metric] = {"n":len(values), "median":statistics.median(values),
                                             "p95":values[max(0, (P95_PERCENTILE * len(values) + PERCENTILE_CEIL_OFFSET) // PERCENT_SCALE - 1)], "max":max(values)}
    if valid:
        first = report["samples"][0]
        report["first_submission_ms"] = first.get("input_to_submission_ms")
        later = [row["input_to_submission_ms"] for row in report["samples"][1:] if row["valid_submission"]]
        report["subsequent_submission_median_ms"] = statistics.median(later) if later else None
    args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({key: value for key, value in report.items() if key != "samples"}, ensure_ascii=False, indent=2))


def read_report(path):
    report = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(report, dict) or not isinstance(report.get("samples"), list):
        raise ValueError("Report must contain a samples list")
    identities = set()
    for row in report["samples"]:
        if not isinstance(row, dict) or type(row.get("sample")) is not int or row["sample"] in identities:
            raise ValueError("Report samples must have unique integer identities")
        identities.add(row["sample"])
        if any(type(row.get(key)) is not bool for key in ["valid_submission", "display_verified"]):
            raise ValueError("Report sample evidence flags must be boolean")
        if "150ms_target_passed" not in row or (row["150ms_target_passed"] is not None and type(row["150ms_target_passed"]) is not bool):
            raise ValueError("Report sample budget must be true, false or null")
        verified = row["valid_submission"] and row["display_verified"]
        if row["display_verified"] != verified or verified != (row["150ms_target_passed"] is not None):
            raise ValueError("Report sample budget and display evidence disagree")
    complete = bool(report["samples"]) and all(row["display_verified"] for row in report["samples"])
    verdict = all(row["150ms_target_passed"] for row in report["samples"]) if complete else None
    if report.get("display_status") != ("verified" if complete else "incomplete") or report.get("150ms_target_passed", "missing") is not verdict:
        raise ValueError("Report aggregate and sample evidence disagree")
    return report


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
    parser.add_argument("--port", type=int, default=DEFAULT_BRP_PORT)
    parser.add_argument("--operation", default="OpenFile")
    parser.add_argument("--launcher-position", nargs=2, type=float)
    parser.add_argument("--cancel-position", nargs=2, type=float, help="Override the default named Cancel control position")
    parser.add_argument("--samples", type=int, default=DEFAULT_SAMPLES)
    parser.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT_SECONDS)
    parser.add_argument("--fixture", type=Path, help="Create a new deterministic directory and exit")
    parser.add_argument("--entries", type=int, default=DEFAULT_ENTRIES)
    args = parser.parse_args()
    if args.fixture:
        if args.entries < 0:
            parser.error("--entries must be nonnegative")
        args.fixture.mkdir(parents=True, exist_ok=False)
        for index in range(args.entries):
            if index % OBSERVER_SAMPLE_INTERVAL == 0:
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
