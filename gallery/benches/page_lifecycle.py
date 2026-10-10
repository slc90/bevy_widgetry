import argparse
import csv
import hashlib
import json
import statistics
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path


PAGES = ["Button", "CheckBox", "ComboBox", "ScrollArea", "ListView", "Tree", "Table", "TextField", "Tooltip", "Window", "Waveform"]
SOURCES = {
    "ComboBox": "widget_gallery::pages::combo_box::ComboBoxDemoSources",
    "ListView": "widget_gallery::pages::list_view::DemoSources",
    "Tree": "widget_gallery::pages::tree::DemoSources",
    "Table": "widget_gallery::pages::table::TableDemoSources",
}


class RpcError(RuntimeError):
    def __init__(self, error):
        self.code = error["code"]
        super().__init__(error)


def rpc(port, method, params):
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}",
        data=json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode(),
        headers={"Content-Type": "application/json"},
    )
    with urllib.request.urlopen(request, timeout=15) as response:
        value = json.load(response)
    if "error" in value:
        raise RpcError(value["error"])
    return value["result"]


def wait_pointer(port, inactive=False):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        state = rpc(port, "brp_extras/pointer_control", {"action": "status"})
        if state["last_error"] is not None:
            raise RuntimeError(state["last_error"])
        complete = state["phase"] == "inactive" if inactive else not state["busy"]
        if complete:
            return
        time.sleep(0.02)
    raise TimeoutError("Pointer 未完成")


def names(port):
    return rpc(port, "world.query", {"data": {"components": ["bevy_ecs::name::Name"]}})


def named(rows, name):
    matches = [row["entity"] for row in rows if row["components"]["bevy_ecs::name::Name"] == name]
    if len(matches) != 1:
        raise RuntimeError(f"Name {name}: {len(matches)} 个匹配")
    return matches[0]


def click_page(port, page):
    rpc(port, "brp_extras/move_mouse", {"position": [88.0, 73.0 + PAGES.index(page) * 50.0]})
    rpc(port, "brp_extras/click_mouse", {"button": "Left"})
    wait_pointer(port)
    return named(names(port), page + "Page")


def owned(port, page):
    if page not in SOURCES:
        return []
    value = rpc(port, "world.get_resources", {"resource": SOURCES[page]})["value"]
    if isinstance(value, list):
        return value
    return value.get("models", value.get("sources", [])) + value.get("roots", [])


def samples(output):
    with (output / "switches.csv").open(encoding="utf-8", newline="") as source:
        return list(csv.DictReader(source))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--port", type=int, default=15702)
    parser.add_argument("--pages", nargs="+", choices=PAGES[1:], default=["ListView"])
    parser.add_argument("--warmup", type=int, default=3)
    parser.add_argument("--cycles", type=int, default=20)
    parser.add_argument("--budget-ms", type=float, default=250.0)
    parser.add_argument("--executable", type=Path, required=True)
    parser.add_argument("--profile", required=True)
    args = parser.parse_args()
    if args.warmup < 0 or args.cycles < 1 or args.budget_ms <= 0:
        parser.error("warmup 非负，cycles 与 budget 必须为正")
    output = args.output.resolve()
    workspace = Path(__file__).resolve().parents[2]
    environment = subprocess.check_output([
        "pwsh", "-NoProfile", "-Command",
        "@{os=(Get-CimInstance Win32_OperatingSystem).Caption; cpu=(Get-CimInstance Win32_Processor).Name; gpu=(Get-CimInstance Win32_VideoController).Name} | ConvertTo-Json -Compress",
    ], text=True, encoding="utf-8")
    with args.executable.open("rb") as executable:
        digest = hashlib.file_digest(executable, "sha256").hexdigest()
    config = dict(vars(args), executable_sha256=digest, environment=json.loads(environment),
                  rustc=subprocess.check_output(["rustc", "--version"], text=True).strip(),
                  head=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=workspace, text=True).strip(),
                  working_tree=subprocess.check_output(["git", "status", "--porcelain"], cwd=workspace, text=True).strip(),
                  window=[1920, 1080], update_mode="desktop_app", rendering="DX12",
                  boundary="First to Last after Waveform Status; excludes CSV I/O, render thread/GPU, BRP RTT and idle",
                  cache="OS/filesystem/asset caches not flushed; first visit recorded separately from warmed switches")
    with (output / "scenario.json").open("x", encoding="utf-8") as target:
        json.dump(config, target, default=str, ensure_ascii=False, indent=2)
    findings = []
    try:
        header = named(names(args.port), "ThemeComboBox")
        for page in args.pages:
            for cycle in range(args.warmup + args.cycles):
                click_page(args.port, "Button")
                before = len(samples(output))
                root = click_page(args.port, page)
                entities = owned(args.port, page)
                if click_page(args.port, page) != root:
                    raise RuntimeError(f"{page} 同页重建")
                click_page(args.port, "Button")
                alive = {row["entity"] for row in rpc(args.port, "world.query", {"data": {}})}
                if root in alive or any(entity in alive for entity in entities):
                    raise RuntimeError(f"{page} owned Entity 残留")
                if page in SOURCES:
                    try:
                        rpc(args.port, "world.get_resources", {"resource": SOURCES[page]})
                    except RpcError as error:
                        if error.code != -23502:
                            raise
                    else:
                        raise RuntimeError(f"{page} Resource 残留")
                if named(names(args.port), "ThemeComboBox") != header:
                    raise RuntimeError("Header identity 改变")
                delta = samples(output)[before:]
                if len(delta) != 2 or delta[0]["to"] != page or delta[1]["to"] != "Button":
                    raise RuntimeError(f"{page} 缺失或重复切页测量")
                if any(row["waveform_sources"] != str(row["to"] == "Waveform").lower() for row in delta):
                    raise RuntimeError("Waveform Sources lifecycle 错误")
                findings.append(dict(page=page, cycle=cycle, warmup=cycle < args.warmup, samples=delta))
    finally:
        rpc(args.port, "brp_extras/pointer_control", {"action": "release"})
        wait_pointer(args.port, inactive=True)
    summary = {}
    for page in args.pages:
        measured = [row for cycle in findings if cycle["page"] == page and not cycle["warmup"] for row in cycle["samples"]]
        summary[page] = {}
        for direction in ["enter", "exit"]:
            values = [float(row["main_update_ms"]) for row in measured if (row["to"] == page) == (direction == "enter")]
            summary[page][direction] = dict(samples=len(values), min_ms=min(values), mean_ms=statistics.mean(values),
                                            max_ms=max(values), budget_ms=args.budget_ms, passed=max(values) <= args.budget_ms)
    report = dict(status="complete", cycles=findings, summary=summary,
                  budget_passed=all(direction["passed"] for page in summary.values() for direction in page.values()))
    with (output / "report.json").open("x", encoding="utf-8") as target:
        json.dump(report, target, ensure_ascii=False, indent=2)
    print(json.dumps(summary, ensure_ascii=False))
    if not report["budget_passed"]:
        raise RuntimeError("切页 CPU budget 未通过")


if __name__ == "__main__":
    main()
