import argparse
import ctypes
from ctypes import wintypes
from datetime import datetime
import json
from pathlib import Path
import platform
import queue
import re
import statistics
import subprocess
import sys
import threading
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
    return value


def complete_lines(log):
    while True:
        position = log.tell()
        line = log.readline()
        if not line:
            return
        if not line.endswith("\n"):
            log.seek(position)
            return
        yield line


def read_operation(log, state):
    for line in complete_lines(log):
        if not re.search(r"operation=OpenFile(?:\s|$)", line):
            continue
        if "发起文件对话框操作" in line:
            if state["requested"] is not None:
                raise RuntimeError("存在其他并行 Open File 请求，无法匹配 sample")
            state["requested"] = datetime.strptime(line[:23], "%Y-%m-%d %H:%M:%S.%f").timestamp()
        elif "文件对话框操作完成" in line:
            state["completed"] = True


def measure(args, events, report, done):
    phase_log = None
    try:
        if args.phases_log:
            phase_log = args.phases_log.open(encoding="utf-8", errors="replace")
            phase_log.seek(0, 2)
        with args.log.open(encoding="utf-8") as log:
            log.seek(0, 2)
            for index in range(args.samples):
                report["active_sample"] = {"index": index}
                deadline = time.monotonic() + args.timeout
                operation = {"requested": None, "completed": False}
                attempts = 0
                while operation["requested"] is None and attempts < 3 and time.monotonic() < deadline:
                    attempts += 1
                    rpc(args.port, "brp_extras/move_mouse", {"position": args.position})
                    rpc(args.port, "world.query", {"data": {}, "filter": {"with": ["bevy_window::window::Window"]}})
                    rpc(args.port, "brp_extras/click_mouse", {"button": "Left"})
                    activation_deadline = min(deadline, time.monotonic() + 2)
                    while time.monotonic() < activation_deadline:
                        read_operation(log, operation)
                        if operation["requested"] is not None:
                            break
                        time.sleep(0.01)
                if operation["requested"] is None:
                    raise TimeoutError("BRP input 未触发 Open File 操作")
                shown = events.get(timeout=max(0.01, deadline - time.monotonic()))
                report["active_sample"]["shown"] = shown
                while time.monotonic() < deadline and not operation["completed"]:
                    read_operation(log, operation)
                    if not operation["completed"]:
                        time.sleep(0.01)
                requested = operation["requested"]
                if not operation["completed"]:
                    raise TimeoutError("未取得操作日志或取消结果，请手动取消 native dialog")
                elapsed = shown["wall_ns"] / 1e6 - requested * 1000
                if elapsed < 0 or elapsed > args.timeout * 1000:
                    raise RuntimeError(f"SHOW 与操作日志无法匹配: {elapsed} ms")
                sample = {"index": index, "launch_attempts": attempts, "request_to_show_ms": elapsed, **shown}
                if phase_log:
                    points = {}
                    while time.monotonic() < deadline:
                        for line in complete_lines(phase_log):
                            if "@@file_dialog_phase_error" in line:
                                raise RuntimeError(line.strip())
                            match = re.search(r"@@file_dialog_phase (\w+) (\d+)", line)
                            if match:
                                if match[1] in points:
                                    raise RuntimeError("phase 重复，存在其他并行 Open File 操作")
                                points[match[1]] = int(match[2])
                        if "show_begin" in points:
                            break
                        time.sleep(0.01)
                    required = ["rfd_enter", "com_begin", "com_end", "object_begin", "object_end", "show_begin"]
                    if any(point not in points for point in required):
                        raise RuntimeError(f"phase 不完整: {points}")
                    if any(points[a] > points[b] for a, b in zip(required, required[1:])):
                        raise RuntimeError("phase clock 倒退或 sample 错配")
                    start_ns = round(requested * 1e9)
                    segments = {
                        "gallery_dispatch": (points["rfd_enter"] - start_ns) / 1e6,
                        "rfd_bookkeeping": ((points["com_begin"] - points["rfd_enter"]) +
                                            (points["object_begin"] - points["com_end"])) / 1e6,
                        "com_init": (points["com_end"] - points["com_begin"]) / 1e6,
                        "com_object": (points["object_end"] - points["object_begin"]) / 1e6,
                        "configure": (points["show_begin"] - points["object_end"]) / 1e6,
                        "native_show": (shown["wall_ns"] - points["show_begin"]) / 1e6,
                    }
                    sample["phase_points_ns"] = points
                    sample["segments_ms"] = segments
                if shown.get("created"):
                    created = shown["created"]
                    sample["request_to_create_ms"] = created["wall_ns"] / 1e6 - requested * 1000
                    sample["create_to_show_ms"] = ((shown["event_ticks"] - created["event_ticks"]) & 0xFFFFFFFF)
                    if phase_log:
                        sample["show_entry_to_create_ms"] = (created["wall_ns"] - points["show_begin"]) / 1e6
                report["samples"].append(sample)
                report.pop("active_sample", None)
                args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
                print(json.dumps(sample), flush=True)
                # worker 日志先于结果 Text 更新，用实际 ECS state 等待 main thread 消费结果。
                while time.monotonic() < deadline:
                    state = rpc(args.port, "world.query", {"data": {"components": ["bevy_ui::widget::text::Text"]}})
                    if not any(entity["components"]["bevy_ui::widget::text::Text"] == "Result: Waiting..."
                               for entity in state["result"]):
                        break
                    time.sleep(0.01)
                else:
                    raise TimeoutError("main thread 未消费文件对话框结果")
                time.sleep(0.2)
        samples = report["samples"]
        subsequent = [value["request_to_show_ms"] for value in samples[1:]]
        report["first_ms"] = samples[0]["request_to_show_ms"]
        report["subsequent_median_ms"] = statistics.median(subsequent)
        report["subsequent_range_ms"] = [min(subsequent), max(subsequent)]
        report["native_thread_ids"] = sorted({value["tid"] for value in samples})
        if phase_log:
            report["segment_medians_ms"] = {
                key: statistics.median(value["segments_ms"][key] for value in samples[1:])
                for key in samples[0]["segments_ms"]
            }
        if all(value.get("created") for value in samples):
            report["create_to_show_median_ms"] = statistics.median(value["create_to_show_ms"] for value in samples[1:])
        if args.expect_reuse and len(report["native_thread_ids"]) != 1:
            raise RuntimeError("regression: native dialog 没有复用同一 thread")
        if args.baseline:
            baseline = json.loads(args.baseline.read_text(encoding="utf-8"))
            for key in ["profile", "os", "cpu", "position", "update_mode", "boundary"]:
                if baseline.get(key) != report[key]:
                    raise ValueError(f"baseline 测量条件不同: {key}")
            if baseline.get("status") != "passed":
                raise ValueError("baseline 必须是完整成功的采样")
            report["baseline_median_ms"] = baseline["subsequent_median_ms"]
            report["improvement_ms"] = report["baseline_median_ms"] - report["subsequent_median_ms"]
            if report["improvement_ms"] <= 0:
                raise RuntimeError("性能预算未满足: 后续 median 未低于 baseline")
        report["status"] = "passed"
    except Exception as error:
        report["status"] = "failed"
        report["error"] = repr(error)
    finally:
        if phase_log:
            phase_log.close()
        done.set()


def main():
    sys.stdout.reconfigure(encoding="utf-8")
    parser = argparse.ArgumentParser(description="Windows Gallery Open File benchmark。先通过 BRP 启动优化构建并切到 Window 页面。默认手动取消每个 native dialog。")
    parser.add_argument("--pid", type=int, required=True)
    parser.add_argument("--log", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--port", type=int, default=15702)
    parser.add_argument("--position", type=float, nargs=2, default=[267, 481])
    parser.add_argument("--samples", type=int, default=10)
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--expect-reuse", action="store_true")
    parser.add_argument("--baseline", type=Path, help="同条件修改前 JSON，要求后续 median 改善")
    parser.add_argument("--phases-log", type=Path, help="临时 rfd 插桩版本的 stderr/MCP process log，取消后读取同一 sample 的 phase markers")
    parser.add_argument("--profile", required=True, help="例如 release，记录实际优化构建")
    args = parser.parse_args()
    if platform.system() != "Windows" or args.pid <= 0 or args.samples < 2 or args.timeout <= 0:
        parser.error("需要 Windows、正数 PID、至少两个 samples、正数 timeout")
    user32 = ctypes.WinDLL("user32", use_last_error=True)
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    callback_type = ctypes.WINFUNCTYPE(None, wintypes.HANDLE, wintypes.DWORD, wintypes.HWND,
                                      ctypes.c_long, ctypes.c_long, wintypes.DWORD, wintypes.DWORD)
    user32.SetWinEventHook.argtypes = [wintypes.DWORD, wintypes.DWORD, wintypes.HMODULE,
                                      callback_type, wintypes.DWORD, wintypes.DWORD, wintypes.DWORD]
    user32.SetWinEventHook.restype = wintypes.HANDLE
    user32.UnhookWinEvent.argtypes = [wintypes.HANDLE]
    user32.UnhookWinEvent.restype = wintypes.BOOL
    user32.GetClassNameW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.GetWindowTextW.argtypes = [wintypes.HWND, wintypes.LPWSTR, ctypes.c_int]
    user32.GetAncestor.argtypes = [wintypes.HWND, wintypes.UINT]
    user32.GetAncestor.restype = wintypes.HWND
    user32.PeekMessageW.argtypes = [ctypes.POINTER(wintypes.MSG), wintypes.HWND,
                                   wintypes.UINT, wintypes.UINT, wintypes.UINT]
    user32.TranslateMessage.argtypes = [ctypes.POINTER(wintypes.MSG)]
    user32.DispatchMessageW.argtypes = [ctypes.POINTER(wintypes.MSG)]
    user32.DispatchMessageW.restype = ctypes.c_ssize_t
    kernel32.GetTickCount64.restype = ctypes.c_ulonglong
    events = queue.Queue()
    seen = set()
    created_windows = {}

    @callback_type
    def on_show(hook, event, hwnd, object_id, child_id, tid, ticks):
        if object_id != 0 or child_id != 0 or not hwnd or user32.GetAncestor(hwnd, 2) != hwnd:
            return
        classname = ctypes.create_unicode_buffer(256)
        title = ctypes.create_unicode_buffer(256)
        user32.GetClassNameW(hwnd, classname, 256)
        user32.GetWindowTextW(hwnd, title, 256)
        if classname.value != "#32770":
            return
        delay = ((kernel32.GetTickCount64() & 0xFFFFFFFF) - ticks) & 0xFFFFFFFF
        # EVENT_OBJECT_SHOW 使用系统 tick，扣除 callback 送达延迟，仍有约 16 ms clock 粒度。
        value = {"hwnd": hwnd, "tid": tid, "wall_ns": time.time_ns() - delay * 1_000_000,
                 "event_ticks": ticks, "callback_delay_ms": delay}
        if event == 0x8000:
            seen.discard(hwnd)
            created_windows[hwnd] = value
        elif event == 0x8002 and title.value == "Open File" and hwnd not in seen:
            seen.add(hwnd)
            value["created"] = created_windows.pop(hwnd, None)
            events.put(value)

    hook = user32.SetWinEventHook(0x8000, 0x8002, None, on_show, args.pid, 0, 0)
    if not hook:
        raise ctypes.WinError(ctypes.get_last_error())
    report = {"status": "running", "samples": [], "pid": args.pid, "profile": args.profile,
              "os": platform.platform(), "cpu": platform.processor(), "log": str(args.log),
              "port": args.port, "position": args.position, "cancellation": "manual",
              "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
              "working_tree": subprocess.check_output(["git", "status", "--porcelain"], text=True),
              "rustc": subprocess.check_output(["rustc", "--version"], text=True).strip(),
              "boundary": "Gallery operation log -> native EVENT_OBJECT_SHOW; not first pixel",
              "clock_error_ms": 16, "cache": "OS / IME / Shell cache not controlled",
              "phases_log": str(args.phases_log) if args.phases_log else None,
              "instrumentation": "rfd thread-local timestamps; flush after cancellation" if args.phases_log else "WinEvent observer only",
              "update_mode": "unchanged desktop_app", "command": subprocess.list2cmdline(sys.argv)}
    done = threading.Event()
    worker = threading.Thread(target=measure, args=(args, events, report, done))
    worker.start()
    message = wintypes.MSG()
    try:
        while not done.is_set():
            while user32.PeekMessageW(ctypes.byref(message), None, 0, 0, 1):
                user32.TranslateMessage(ctypes.byref(message))
                user32.DispatchMessageW(ctypes.byref(message))
            time.sleep(0.002)
    finally:
        worker.join()
        if not user32.UnhookWinEvent(hook):
            raise ctypes.WinError(ctypes.get_last_error())
        args.output.write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(report, ensure_ascii=False), flush=True)
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
