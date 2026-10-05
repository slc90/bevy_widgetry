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


def collect(args):
    windows = rpc(args.port, "world.query", {"data": {}, "filter": {
        "with": ["bevy_window::window::PrimaryWindow", "bevy_window::window::Window"]}})
    if len(windows) != 1:
        raise RuntimeError("Expected exactly one PrimaryWindow for launcher input")
    primary = windows[0]["entity"]
    for _ in range(args.samples):
        previous = {event["sample"] for event in events(args.events) if "sample" in event}
        # 未指定 window 时 BRP 会复用上次 cursor window，Cancel 后该 window 已失效。
        rpc(args.port, "brp_extras/move_mouse", {"position": args.launcher_position, "window": primary})
        # BRP 输入按 update 入队。先让 pointer hover 收敛，避免同帧 Click 命中旧位置。
        time.sleep(0.15)
        rpc(args.port, "brp_extras/click_mouse", {"button": "Left", "window": primary})
        deadline = time.monotonic() + args.timeout
        sample = None
        while time.monotonic() < deadline:
            trace = events(args.events)
            started = [event for event in trace if event.get("event") == "activate"
                       and event["sample"] not in previous]
            if len(started) > 1:
                raise RuntimeError("Concurrent launcher activations invalidate sequential collection")
            if started:
                sample = started[0]["sample"]
                if started[0]["operation"] != args.operation or started[0]["t_input"] is None:
                    raise RuntimeError("Launcher or App input boundary did not match the sample")
                points = {event["event"]: event for event in trace if event.get("sample") == sample}
                if "first_content_frame" in points and "final_projection" in points:
                    content = points["first_content_frame"]
                    break
            time.sleep(0.05)
        else:
            raise TimeoutError(f"Missing content/projection evidence for sample {sample}")
        window = content["native_window"]
        rpc(args.port, "brp_extras/move_mouse", {"position": args.cancel_position, "window": window})
        time.sleep(0.15)
        rpc(args.port, "brp_extras/click_mouse", {"button": "Left", "window": window})
        # Wait for actual owned cleanup rather than mutating ECS to reset the fixture.
        while time.monotonic() < deadline:
            windows = rpc(args.port, "world.query", {"data": {}, "filter": {
                "with": ["bevy_window::window::Window"]}})
            if not any(row["entity"] == window for row in windows):
                break
            time.sleep(0.05)
        else:
            raise TimeoutError("Cancel did not reclaim the native Window")
        print(f"Collected sample {sample}", flush=True)


def analyze(args):
    trace = events(args.events)
    protocols = [event for event in trace if event.get("event") == "protocol"]
    if len(protocols) != 1 or protocols[0].get("protocol") != 1:
        raise ValueError("Expected one FileDialog protocol-v1 trace")
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
        required = {"activate", "scene", "camera_ready", "cpu_content_ready", "first_content_frame", "final_projection"}
        if missing := required - points.keys():
            row["invalid_reason"] = f"Missing points: {sorted(missing)}"
            continue
        activation, scene, content = points["activate"], points["scene"], points["first_content_frame"]
        row.update(operation=activation["operation"], root=scene["root"], session=scene["session"],
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
            row["activate_to_presented_ms"] = (display["ns"] - activation["t_activate"]) / 1e6
    valid = [row for row in report["samples"] if row["valid_submission"]]
    display = [row for row in valid if row["display_verified"]]
    report["display_status"] = "verified" if display and len(display) == len(report["samples"]) else "incomplete"
    report["150ms_target_passed"] = all(row["input_to_presented_ms"] <= 150 for row in display) if report["display_status"] == "verified" else None
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
        if args.entries < 1:
            parser.error("--entries must be positive")
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
        collect(args)
    analyze(args)


if __name__ == "__main__":
    main()
