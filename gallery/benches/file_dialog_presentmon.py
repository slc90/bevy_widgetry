"""Correlate PresentMon 2.3.1 --v1_metrics --qpc_time with Gallery content frames.

Requires one baseline primary swapchain and exactly two native windows at the
content frame. Ambiguous, dropped and unavailable display events are invalid.
"""
import argparse
import csv
from decimal import Decimal, InvalidOperation
import json
from pathlib import Path


FIELDS = {"ProcessID", "SwapChainAddress", "QPCTime", "Dropped", "msUntilDisplayed"}


def load_rows(stream):
    reader = csv.DictReader(stream)
    if not FIELDS.issubset(reader.fieldnames or []):
        raise ValueError("PresentMon schema must be v1_metrics with qpc_time")
    return list(reader)


def clock(protocol):
    calibration = protocol.get("qpc_calibration")
    if not calibration or calibration["frequency"] <= 0:
        raise ValueError("Missing or invalid QPC calibration")
    before, after = calibration["before_ns"], calibration["after_ns"]
    if before < 0 or after < before:
        raise ValueError("Invalid calibration bracket")
    midpoint = (before + after) // 2
    uncertainty = (after - before + 1) // 2 + (1000000000 + calibration["frequency"] - 1) // calibration["frequency"]

    def mapped(qpc):
        return midpoint + (int(qpc) - calibration["qpc"]) * 1000000000 // calibration["frequency"]

    if check := protocol.get("clock_check"):
        if check["frequency"] != calibration["frequency"] or check["after_ns"] < check["before_ns"]:
            raise ValueError("Invalid clock check")
        predicted = mapped(check["qpc"])
        deviation = max(abs(predicted - check["before_ns"]), abs(predicted - check["after_ns"]))
        if deviation > 100000:
            raise ValueError("QPC/App clock drift exceeds 0.1ms")
        uncertainty += deviation
    return mapped, uncertainty


def correlate(protocol, content, rows, primary_surface, artifact):
    mapped, uncertainty = clock(protocol)
    if content.get("window_count") != 2 or not content.get("hwnd"):
        raise ValueError("Exclusive mapping requires exactly two windows and a dialog HWND")
    start, end = content["render_start_ns"], content["ns"]
    if end < start:
        raise ValueError("Invalid render interval")
    candidates = [row for row in rows
                  if int(row["ProcessID"]) == protocol["process_id"]
                  and row["SwapChainAddress"] != primary_surface
                  and start - uncertainty <= mapped(row["QPCTime"]) <= end + uncertainty]
    if len(candidates) != 1:
        raise ValueError(f"Expected exactly one dialog present in render interval, found {len(candidates)}")
    row = candidates[0]
    try:
        latency = Decimal(row["msUntilDisplayed"])
    except InvalidOperation:
        raise ValueError("Frame was not displayed: missing display latency") from None
    if row["Dropped"] != "0" or not latency.is_finite() or latency < 0:
        raise ValueError("Frame was not displayed")
    return {**{key: content[key] for key in ["sample", "root", "session", "native_window", "camera", "app_frame"]},
            "clock_id": protocol["clock_id"], "endpoint": "displayed",
            "source": "PresentMon-2.3.1-v1-QPCTime-plus-msUntilDisplayed",
            "artifact": str(artifact), "swapchain": row["SwapChainAddress"],
            "present_qpc": int(row["QPCTime"]), "hwnd": content["hwnd"],
            "ns": mapped(row["QPCTime"]) + int(latency * 1000000),
            "uncertainty_ns": uncertainty}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--events", type=Path, required=True)
    parser.add_argument("--csv", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    trace = [json.loads(line) for line in args.events.read_text(encoding="utf-8").splitlines()]
    protocols = [event for event in trace if event.get("event") == "protocol"]
    if len(protocols) != 1:
        raise ValueError("Expected exactly one protocol")
    protocol = protocols[0]
    checks = [event for event in trace if event.get("event") == "clock_check"]
    if len(checks) != 1:
        raise ValueError("Expected one post-measurement clock check")
    protocol = dict(protocol, clock_check=checks[0]["qpc_calibration"])
    mapped, _ = clock(protocol)
    first_input = min(event["t_input"] for event in trace if event.get("event") == "activate" and event["t_input"] is not None)
    with args.csv.open(encoding="utf-8-sig", newline="") as stream:
        rows = load_rows(stream)
    baseline = {row["SwapChainAddress"] for row in rows
                if int(row["ProcessID"]) == protocol["process_id"] and mapped(row["QPCTime"]) < first_input}
    if len(baseline) != 1:
        raise ValueError(f"Expected exactly one primary baseline swapchain, found {len(baseline)}")
    primary = baseline.pop()
    results, invalid = [], []
    for content in trace:
        if content.get("event") != "first_content_frame":
            continue
        try:
            results.append(correlate(protocol, content, rows, primary, args.csv.resolve()))
        except ValueError as error:
            invalid.append({"sample": content["sample"], "reason": str(error)})
    args.output.write_text("".join(json.dumps(row) + "\n" for row in results), encoding="utf-8")
    mapping = {"primary_swapchain": primary, "displayed": len(results), "invalid": invalid,
               "schema": "PresentMon-2.3.1-v1_metrics-qpc_time"}
    args.output.with_suffix(".mapping.json").write_text(json.dumps(mapping, indent=2), encoding="utf-8")
    print(json.dumps(mapping, indent=2))


if __name__ == "__main__":
    main()
