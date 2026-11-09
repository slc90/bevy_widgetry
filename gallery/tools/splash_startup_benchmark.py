"""Measure normal EXE startup from passive logs; contact BRP only after handoff.

Build the release binary first, then run:
python gallery/tools/splash_startup_benchmark.py --output target/splash-startup
Keep the desktop unobstructed and avoid mouse/keyboard input during each launch.
"""

import argparse
from datetime import datetime
import json
import os
from pathlib import Path
import platform
import socket
import subprocess
import time
import urllib.request


STARTED = "Gallery 启动配置开始"
SHOWN = "Gallery Splash 首帧已安全显示"
COMPLETED = "Gallery 已完成独立 Splash 交接"
TIMESTAMP_FORMAT = "%Y-%m-%d %H:%M:%S.%f"
TIMEOUT_SECONDS = 60
POLL_SECONDS = 0.05


def timestamp(lines, marker):
    line = next(line for line in lines if marker in line)
    return datetime.strptime(line[:23], TIMESTAMP_FORMAT)


def run(binary, output, index, port):
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", port))
    logs = binary.parent / "logs"
    existing = set(logs.glob("*.log"))
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("GALLERY_")}
    env["BRP_EXTRAS_PORT"] = str(port)
    log = None
    with (output / f"run-{index}.stdio.log").open("wb") as console:
        launched = datetime.now()
        process = subprocess.Popen([str(binary)], env=env, stdout=console,
                                   stderr=console, cwd=Path(__file__).resolve().parents[2])
        try:
            deadline = time.monotonic() + TIMEOUT_SECONDS
            while time.monotonic() < deadline:
                if process.poll() is not None:
                    raise RuntimeError(f"Gallery exited before handoff: {process.returncode}")
                if log is None:
                    candidates = set(logs.glob("*.log")) - existing
                    if len(candidates) > 1:
                        raise RuntimeError("Multiple new Gallery logs; run without other instances")
                    log = next(iter(candidates), None)
                if log is not None:
                    content = log.read_text(encoding="utf-8", errors="replace")
                    if COMPLETED in content:
                        break
                time.sleep(POLL_SECONDS)
            else:
                raise TimeoutError(f"No Splash handoff within {TIMEOUT_SECONDS}s; log={log}")

            # No request reaches the app until both Splash display and handoff are logged.
            request = urllib.request.Request(
                f"http://127.0.0.1:{port}",
                data=json.dumps({"jsonrpc": "2.0", "id": 1,
                                 "method": "brp_extras/shutdown"}).encode(),
                headers={"Content-Type": "application/json"},
            )
            opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
            with opener.open(request, timeout=TIMEOUT_SECONDS) as response:
                result = json.load(response)
            if "error" in result:
                raise RuntimeError(result["error"])
            if process.wait(timeout=TIMEOUT_SECONDS) != 0:
                raise RuntimeError(f"Gallery failed: {process.returncode}")
            content = log.read_text(encoding="utf-8")
            (output / f"run-{index}.log").write_text(content, encoding="utf-8")
            if " ERROR " in content or "panicked at" in content:
                raise RuntimeError(f"Gallery error; inspect {log}")
            lines = content.splitlines()
            started = timestamp(lines, STARTED)
            shown = timestamp(lines, SHOWN)
            completed = timestamp(lines, COMPLETED)
            return {
                "log": str(log),
                "launch_to_shown_ms": (shown - launched).total_seconds() * 1000,
                "first_log_to_shown_ms": (shown - started).total_seconds() * 1000,
                "shown_to_handoff_ms": (completed - shown).total_seconds() * 1000,
            }
        finally:
            if process.poll() is None:
                process.kill()
                process.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path,
                        default=Path(__file__).resolve().parents[2] / "target/release/widget_gallery.exe")
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--port", type=int, default=15982)
    args = parser.parse_args()
    if args.repeat < 1 or not 1024 <= args.port <= 65534 or args.port == 15703:
        parser.error("repeat must be positive; port must be 1024..65534 except render port 15703")
    binary = args.binary.resolve(strict=True)
    args.output.mkdir(parents=True, exist_ok=True)
    report = {
        "binary": str(binary), "platform": platform.platform(),
        "timing": "launch request/first app log to SplashShown log; not desktop pixel latency",
        "conditions": "normal splash, existing config, no BRP until handoff; OS/shader caches retained",
        "binary_modified_ns": binary.stat().st_mtime_ns,
        "runs": [],
    }
    for index in range(1, args.repeat + 1):
        result = run(binary, args.output, index, args.port)
        report["runs"].append(result)
        (args.output / "summary.json").write_text(
            json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
        print(json.dumps(result, ensure_ascii=False), flush=True)


if __name__ == "__main__":
    main()
