"""Repeat optimized Gallery processes and real BRP FileDialog input paths.

Build first. Captures App trace and PresentMon without concurrent compilation.
No measurements include launch, HTTP, screenshot or export durations.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import socket
import time
from types import SimpleNamespace

from file_dialog import analyze, click_named, collect, named_geometry, point, rpc


def wait_app(port, process):
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise RuntimeError(f"Gallery exited with {process.returncode}")
        try:
            response = rpc(port, "benchmark/file_dialog_trace", {"from": 0})
            if response["events"][0]["process_id"] != process.pid:
                raise RuntimeError("BRP port belongs to a different process")
            primary = rpc(port, "world.query", {"data": {}, "filter": {"with": ["bevy_window::window::PrimaryWindow"]}})
            if len(primary) == 1:
                return primary[0]["entity"]
        except (ConnectionError, OSError):
            pass
        time.sleep(0.1)
    raise TimeoutError("Gallery did not become interactive")


def threads(pid):
    command = f"$p = Get-Process -Id {int(pid)}; @{{threads=$p.Threads.Count; working_set=$p.WorkingSet64}} | ConvertTo-Json -Compress"
    return json.loads(subprocess.check_output(["pwsh", "-NoProfile", "-Command", command], text=True))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--fixture", type=Path, required=True)
    parser.add_argument("--presentmon", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=1, help="Independent Gallery processes")
    parser.add_argument("--samples", type=int, default=1, help="New dialogs per process")
    parser.add_argument("--operation", default="OpenFile", choices=["OpenFile", "OpenFiles", "SelectFolder", "SelectFolders", "OpenImage", "SaveFile"])
    parser.add_argument("--port", type=int, default=15881)
    parser.add_argument("--modal", action="store_true")
    parser.add_argument("--io-delay-ms", type=int)
    parser.add_argument("--io-gate", action="store_true")
    parser.add_argument("--navigate", action="store_true")
    parser.add_argument("--scrolls", type=int, default=0)
    parser.add_argument("--quiet-wait-ms", type=int, default=0, help="Wait after launcher input without polling BRP")
    parser.add_argument("--frames", action="store_true")
    parser.add_argument("--screenshot", action="store_true")
    parser.add_argument("--control", action="store_true", help="Disable content probes, retain identical frame timing for overhead comparison")
    args = parser.parse_args()
    if args.runs < 1 or args.samples < 1 or args.scrolls < 0 or args.quiet_wait_ms < 0 or (args.io_delay_ms is not None and args.io_delay_ms < 0):
        parser.error("Run/sample counts must be positive and other counts nonnegative")
    if not args.fixture.is_dir() or not args.presentmon.is_file():
        parser.error("Fixture and PresentMon must already exist")
    version = subprocess.run([str(args.presentmon.resolve()), "--help"], capture_output=True, text=True, creationflags=subprocess.CREATE_NO_WINDOW)
    if "PresentMon 2.3.1" not in version.stdout + version.stderr:
        parser.error("This adapter requires PresentMon 2.3.1")
    with socket.socket() as check:
        if check.connect_ex(("127.0.0.1", args.port)) == 0:
            parser.error("BRP port is already in use")
    args.output.mkdir(parents=True, exist_ok=False)
    workspace = Path(__file__).resolve().parents[2]
    binary = workspace / "target/release/widget_gallery.exe"
    config = vars(args).copy()
    with binary.open("rb") as executable:
        binary_hash = hashlib.file_digest(executable, "sha256").hexdigest()
    config.update(binary_sha256=binary_hash,
                  head=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=workspace, text=True).strip(),
                  working_tree=subprocess.check_output(["git", "status", "--porcelain=v1"], cwd=workspace, text=True),
                  profile="release", cache="OS/filesystem caches not flushed")
    args.output.joinpath("config.json").write_text(json.dumps(config, default=str, indent=2), encoding="utf-8")
    reports = []
    for run in range(args.runs):
        output = (args.output / f"run-{run + 1:03}").resolve()
        output.mkdir()
        env = os.environ.copy()
        env.update(BRP_EXTRAS_PORT=str(args.port), GALLERY_FILE_DIALOG_BENCH_OUTPUT=str(output),
                   GALLERY_FILE_DIALOG_BENCH_FIXTURE=str(args.fixture.resolve()))
        for name in ["GALLERY_FILE_DIALOG_BENCH_FRAMES", "GALLERY_FILE_DIALOG_BENCH_IO_DELAY_MS", "GALLERY_FILE_DIALOG_BENCH_IO_GATE", "GALLERY_FILE_DIALOG_BENCH_CONTENT"]:
            env.pop(name, None)
        if args.frames:
            env["GALLERY_FILE_DIALOG_BENCH_FRAMES"] = "1"
        if args.control:
            env["GALLERY_FILE_DIALOG_BENCH_CONTENT"] = "off"
        if args.io_delay_ms is not None:
            env["GALLERY_FILE_DIALOG_BENCH_IO_DELAY_MS"] = str(args.io_delay_ms)
        if args.io_gate:
            env["GALLERY_FILE_DIALOG_BENCH_IO_GATE"] = "1"
        flags = subprocess.CREATE_NO_WINDOW
        session = f"WidgetryFD08-{os.getpid()}-{run}"
        with (output / "gallery.log").open("w", encoding="utf-8") as log, (output / "presentmon.log").open("w", encoding="utf-8") as pm_log:
            process = subprocess.Popen([str(binary)], cwd=workspace, env=env, stdout=log, stderr=log, creationflags=flags)
            pm = None
            ready = False
            try:
                primary = wait_app(args.port, process)
                ready = True
                pm = subprocess.Popen([str(args.presentmon.resolve()), "--process_id", str(process.pid),
                    "--session_name", session, "--no_console_stats", "--no_track_input", "--v1_metrics", "--qpc_time",
                    "--output_file", str(output / "present.csv")], stdout=pm_log, stderr=pm_log, creationflags=flags)
                time.sleep(0.3)
                click_named(args.port, "WindowNav", primary)
                if args.modal:
                    click_named(args.port, "GalleryFileDialogModality", primary)
                launcher = point(named_geometry(args.port, f"GalleryFileDialog{args.operation}Launcher")[0])
                baseline = threads(process.pid)
                baseline_world = rpc(args.port, "benchmark/file_dialog_trace", {"from": 0})["resources"]
                collection = SimpleNamespace(port=args.port, samples=args.samples, timeout=30, operation=args.operation,
                    launcher_position=launcher, launcher_name=f"GalleryFileDialog{args.operation}Launcher",
                    cancel_position=[959, 673], events=output / "events.jsonl",
                    navigate=args.navigate, scrolls=args.scrolls, quiet_wait_ms=args.quiet_wait_ms,
                    cancel_loading=args.io_gate, screenshot=args.screenshot and not args.control,
                    control=args.control, thread_observer=lambda: threads(process.pid), expected_modal=args.modal)
                collect(collection)
                if args.io_gate:
                    rpc(args.port, "benchmark/file_dialog_release_io", {})
                    time.sleep(0.5)
                cleanup_deadline = time.monotonic() + 10
                while True:
                    cleanup = rpc(args.port, "benchmark/file_dialog_trace", {"from": 0})["resources"]
                    settled = not any(cleanup[key] for key in ["active_sessions", "queued", "retained_snapshots", "rows"])
                    settled = settled and all(cleanup[key] == baseline_world[key] for key in ["windows", "cameras"])
                    if cleanup["error"] is not None or cleanup["workers"] > 3:
                        raise RuntimeError(f"Runtime resource contract failed: {cleanup}")
                    if settled:
                        break
                    if time.monotonic() > cleanup_deadline:
                        raise TimeoutError(f"Resources did not settle: {cleanup}")
                    time.sleep(0.1)
                (output / "process-resources.json").write_text(json.dumps({"baseline":baseline,"baseline_world":baseline_world,
                    "final":threads(process.pid),"cleanup":cleanup}, indent=2), encoding="utf-8")
            finally:
                try:
                    if process.poll() is None:
                        try:
                            if ready:
                                if args.io_gate:
                                    rpc(args.port, "benchmark/file_dialog_release_io", {})
                                rpc(args.port, "benchmark/file_dialog_export", {})
                                rpc(args.port, "brp_extras/shutdown", {})
                                process.wait(timeout=15)
                            else:
                                process.terminate()
                        finally:
                            if process.poll() is None:
                                process.terminate()
                            process.wait(timeout=15)
                finally:
                    if pm is not None:
                        subprocess.run([str(args.presentmon.resolve()), "--session_name", session, "--terminate_existing_session"],
                                       stdout=pm_log, stderr=pm_log, creationflags=flags, check=True)
                        pm.wait(timeout=15)
        adapter = workspace / "gallery/benches/file_dialog_presentmon.py"
        if not args.control:
            subprocess.run([os.sys.executable, str(adapter), "--events", str(output / "events.jsonl"),
                "--csv", str(output / "present.csv"), "--output", str(output / "presentation.jsonl")], check=True)
        analyze(SimpleNamespace(events=output / "events.jsonl", presentation=None if args.control else output / "presentation.jsonl", output=output / "report.json"))
        report = json.loads((output / "report.json").read_text(encoding="utf-8"))
        reports.append({"run":run + 1,"report":str(output / "report.json"),"samples":report["samples"]})
        args.output.joinpath("runs.json").write_text(json.dumps(reports, indent=2), encoding="utf-8")
        print(f"Run {run + 1}/{args.runs} complete", flush=True)


if __name__ == "__main__":
    main()
