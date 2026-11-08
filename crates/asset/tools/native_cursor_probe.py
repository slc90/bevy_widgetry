"""Validate an existing application's real Windows OS cursors using physical client pixels."""

import argparse
import ctypes as C
from ctypes import wintypes as W
import hashlib
import json
from pathlib import Path
import re
import sys
import time

import numpy as np
from PIL import Image


DPI_AWARENESS_PER_MONITOR_V2 = -4
CURSOR_SHOWING = 1
DIB_RGB_COLORS = 0
MONOCHROME_MASK_PLANES = 2
BITMAP_PLANES = 1
BITMAP_BITS_PER_PIXEL = 32
BGRA_CHANNELS = 4
RGB_CHANNELS = 3
ALPHA_CHANNEL = 3
RGBA_TO_BGRA = [2, 1, 0, 3]
PREMULTIPLICATION_ROUNDING = (0, 127)
MAX_CHANNEL_VALUE = 255
CAPTURE_POLL_SECONDS = .025
PREVIEW_PADDING = 10
MILLISECONDS_PER_SECOND = 1000
FINAL_PREVIEW_FRAME_MS = 25
MAX_CONTACT_FRAMES = 12
EXTENDED_WINDOW_STYLE_INDEX = -20
TOPMOST_STYLE = 8
HWND_TOPMOST = -1
HWND_NOTOPMOST = -2
RESTORE_WINDOW_FLAGS = 0x43
POINTER_NUDGE = 2
NUDGE_SETTLE_SECONDS = .04
HOVER_SETTLE_SECONDS = .5
CAPTURE_ANIMATION_CYCLES = 2
CAPTURE_MARGIN_SECONDS = .2

if sys.platform != "win32":
    raise SystemExit("native_cursor_probe.py requires an interactive Windows desktop")

user = C.WinDLL("user32", use_last_error=True)
gdi = C.WinDLL("gdi32", use_last_error=True)
user.SetProcessDpiAwarenessContext(C.c_void_p(DPI_AWARENESS_PER_MONITOR_V2))
HANDLE = C.c_void_p


class CURSORINFO(C.Structure):
    _fields_ = [("cbSize", W.DWORD), ("flags", W.DWORD), ("hCursor", HANDLE), ("pos", W.POINT)]


class ICONINFO(C.Structure):
    _fields_ = [("fIcon", W.BOOL), ("xHotspot", W.DWORD), ("yHotspot", W.DWORD),
                ("hbmMask", HANDLE), ("hbmColor", HANDLE)]


class BITMAP(C.Structure):
    _fields_ = [("type", W.LONG), ("width", W.LONG), ("height", W.LONG), ("stride", W.LONG),
                ("planes", W.WORD), ("bits", W.WORD), ("data", HANDLE)]


class HEADER(C.Structure):
    _fields_ = [("size", W.DWORD), ("width", W.LONG), ("height", W.LONG), ("planes", W.WORD),
                ("bits", W.WORD), ("compression", W.DWORD), ("sizeImage", W.DWORD),
                ("x", W.LONG), ("y", W.LONG), ("used", W.DWORD), ("important", W.DWORD)]


def api(dll, name, result, *args):
    fn = getattr(dll, name)
    fn.restype, fn.argtypes = result, args
    return fn


get_cursor = api(user, "GetCursorInfo", W.BOOL, C.POINTER(CURSORINFO))
get_icon = api(user, "GetIconInfo", W.BOOL, HANDLE, C.POINTER(ICONINFO))
get_object = api(gdi, "GetObjectW", C.c_int, HANDLE, C.c_int, HANDLE)
delete_object = api(gdi, "DeleteObject", W.BOOL, HANDLE)
get_dc = api(user, "GetDC", HANDLE, W.HWND)
release_dc = api(user, "ReleaseDC", C.c_int, W.HWND, HANDLE)
get_dib = api(gdi, "GetDIBits", C.c_int, HANDLE, HANDLE, W.UINT, W.UINT, HANDLE, HANDLE, W.UINT)
set_pos = api(user, "SetCursorPos", W.BOOL, C.c_int, C.c_int)
get_pos = api(user, "GetCursorPos", W.BOOL, C.POINTER(W.POINT))
find_window = api(user, "FindWindowW", W.HWND, W.LPCWSTR, W.LPCWSTR)
client_to_screen = api(user, "ClientToScreen", W.BOOL, W.HWND, C.POINTER(W.POINT))
get_client_rect = api(user, "GetClientRect", W.BOOL, W.HWND, C.POINTER(W.RECT))
window_from_point = api(user, "WindowFromPoint", W.HWND, W.POINT)
is_child = api(user, "IsChild", W.BOOL, W.HWND, W.HWND)
foreground = api(user, "SetForegroundWindow", W.BOOL, W.HWND)
get_foreground = api(user, "GetForegroundWindow", W.HWND)
is_window = api(user, "IsWindow", W.BOOL, W.HWND)
set_window_pos = api(user, "SetWindowPos", W.BOOL, W.HWND, W.HWND, C.c_int, C.c_int, C.c_int, C.c_int, W.UINT)
get_window_style = api(user, "GetWindowLongW", W.LONG, W.HWND, C.c_int)


def check(ok, operation):
    if not ok:
        raise OSError(C.get_last_error(), operation)
    return ok


def snapshot():
    ci = CURSORINFO(cbSize=C.sizeof(CURSORINFO))
    check(get_cursor(C.byref(ci)), "GetCursorInfo")
    ii = ICONINFO()
    check(get_icon(ci.hCursor, C.byref(ii)), "GetIconInfo")
    try:
        bm = BITMAP()
        check(get_object(ii.hbmColor or ii.hbmMask, C.sizeof(bm), C.byref(bm)), "GetObject")
        height = bm.height if ii.hbmColor else bm.height // MONOCHROME_MASK_PLANES
        info = {"hotspot": [ii.xHotspot, ii.yHotspot], "size": [bm.width, height],
                "position": [ci.pos.x, ci.pos.y], "visible": bool(ci.flags & CURSOR_SHOWING), "handle": ci.hCursor}
        if not ii.hbmColor:
            return info, None
        header = HEADER(size=C.sizeof(HEADER), width=bm.width, height=-height, planes=BITMAP_PLANES, bits=BITMAP_BITS_PER_PIXEL)
        pixels = C.create_string_buffer(bm.width * height * BGRA_CHANNELS)
        dc = check(get_dc(None), "GetDC")
        try:
            check(get_dib(dc, ii.hbmColor, 0, height, pixels, C.byref(header), DIB_RGB_COLORS), "GetDIBits")
        finally:
            release_dc(None, dc)
        image = Image.frombytes("RGBA", (bm.width, height), pixels.raw, "raw", "BGRA")
        info["sha256"] = hashlib.sha256(pixels.raw).hexdigest()
        info["bitmap_format"] = "BGRA8 from GetDIBits"
        return info, image
    finally:
        if ii.hbmColor:
            delete_object(ii.hbmColor)
        if ii.hbmMask:
            delete_object(ii.hbmMask)


def durations(item):
    result = item.get("durations_ms", [item.get("frame_duration_ms", 0)] * item["frames"])
    if not isinstance(result, list) or len(result) != item["frames"] or any(type(v) is not int or v <= 0 for v in result):
        raise ValueError("Every frame needs a positive duration in milliseconds")
    return result


def load_targets(path):
    targets = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(targets, list) or not targets:
        raise ValueError("Targets must be a nonempty JSON array")
    for ordinal, target in enumerate(targets):
        if not isinstance(target, dict):
            raise ValueError(f"Target {ordinal} must be an object")
        name = target.get("name")
        if not isinstance(name, str) or not name.strip():
            raise ValueError(f"Target {ordinal} requires a semantic name")
        for field in ("frame_size", "hotspot", "position"):
            value = target.get(field)
            if not isinstance(value, list) or len(value) != 2 or any(type(v) is not int for v in value):
                raise ValueError(f"{name}: {field} must contain two integers")
        width, height = target["frame_size"]
        if width <= 0 or height <= 0:
            raise ValueError(f"{name}: frame_size must be positive")
        if any(v < 0 for v in target["position"]):
            raise ValueError(f"{name}: position is outside the client area")
        if not (0 <= target["hotspot"][0] < width and 0 <= target["hotspot"][1] < height):
            raise ValueError(f"{name}: hotspot is outside the frame")
        for field in ("columns", "frames"):
            if type(target.get(field)) is not int or target[field] <= 0:
                raise ValueError(f"{name}: {field} must be a positive integer")
        if type(target.get("animated", True)) is not bool:
            raise ValueError(f"{name}: animated must be a boolean")
        durations(target)
        atlas = target.get("atlas")
        if not isinstance(atlas, str) or not atlas:
            raise ValueError(f"{name}: atlas must be a PNG path relative to the target JSON")
        with Image.open(path.resolve().parent / atlas) as source:
            if source.format != "PNG" or source.width < target["columns"] * width or source.height < ((target["frames"] - 1) // target["columns"] + 1) * height:
                raise ValueError(f"{name}: atlas dimensions do not contain all declared frames")
    return targets


def source_hashes(target, directory):
    with Image.open(directory / target["atlas"]) as image:
        source = image.convert("RGBA")
    width, height = target["frame_size"]
    expected = set()
    for index in range(target["frames"]):
        x, y = index % target["columns"] * width, index // target["columns"] * height
        rgba = np.array(source.crop((x, y, x + width, y + height)), dtype=np.uint16)
        expected.add(hashlib.sha256(rgba[:, :, RGBA_TO_BGRA].astype(np.uint8).tobytes()).hexdigest())
        for rounding in PREMULTIPLICATION_ROUNDING:
            converted = rgba.copy()
            converted[:, :, :RGB_CHANNELS] = (rgba[:, :, :RGB_CHANNELS] * rgba[:, :, ALPHA_CHANNEL:BGRA_CHANNELS] + rounding) // MAX_CHANNEL_VALUE
            expected.add(hashlib.sha256(converted[:, :, RGBA_TO_BGRA].astype(np.uint8).tobytes()).hexdigest())
    return expected


def folder(index, name):
    slug = re.sub(r"[^A-Za-z0-9_-]+", "-", name).strip("-") or "cursor"
    return f"{index:03d}-{slug}"


def collect(seconds):
    records, frames = [], []
    start = time.perf_counter()
    while time.perf_counter() - start < seconds:
        info, frame = snapshot()
        info["elapsed"] = time.perf_counter() - start
        records.append(info)
        if frame is not None:
            frames.append((frame, info["elapsed"]))
        time.sleep(CAPTURE_POLL_SECONDS)
    return records, frames


def save(out, records, frames):
    out.mkdir(parents=True, exist_ok=True)
    (out / "native.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
    if frames:
        size = (max(f.width for f, _ in frames) + PREVIEW_PADDING * 2, max(f.height for f, _ in frames) + PREVIEW_PADDING * 2)
        previews, timing = [], []
        for i, (frame, elapsed) in enumerate(frames):
            canvas = Image.new("RGBA", size, "#24283D")
            canvas.alpha_composite(frame, (PREVIEW_PADDING, PREVIEW_PADDING))
            previews.append(canvas)
            timing.append(max(1, round((frames[i + 1][1] - elapsed) * MILLISECONDS_PER_SECOND)) if i + 1 < len(frames) else FINAL_PREVIEW_FRAME_MS)
        previews[0].save(out / "native.apng", save_all=True, append_images=previews[1:], duration=timing, loop=0, disposal=0, blend=0)
        count = min(len(previews), MAX_CONTACT_FRAMES)
        strip = Image.new("RGBA", (count * size[0], size[1]))
        for i in range(count):
            strip.paste(previews[i * len(previews) // count], (i * size[0], 0))
        strip.convert("RGB").save(out / "native-contact.png")
    print(json.dumps({"output": str(out), "samples": len(records),
                      "unique": len({r.get("sha256", r["handle"]) for r in records}),
                      "sizes": sorted({tuple(r["size"]) for r in records}),
                      "hotspots": sorted({tuple(r["hotspot"]) for r in records}),
                      "visible": all(r["visible"] for r in records)}))


def validate_capture(target, records):
    name = target["name"]
    if not records:
        raise ValueError(f"{name}: No native samples captured")
    if not all(r["visible"] for r in records):
        raise ValueError(f"{name}: native cursor is not visible")
    if not all(r["size"] == target["frame_size"] for r in records):
        raise ValueError(f"{name}: native cursor size differs from {target['frame_size']}")
    if not all(r["hotspot"] == target["hotspot"] for r in records):
        raise ValueError(f"{name}: native cursor hotspot differs from {target['hotspot']}")
    unique = len({r.get("sha256") for r in records})
    if target.get("animated", True) and unique < 2:
        raise ValueError(f"{name}: native cursor did not animate")
    if not target.get("animated", True) and unique != 1:
        raise ValueError(f"{name}: static native cursor pixels changed")
    if not all(r["matches_source_frame"] for r in records):
        raise ValueError(f"{name}: native pixels do not match the semantic atlas")


def capture_targets(args):
    targets = load_targets(args.targets)
    hwnd = find_window(None, args.title)
    if not hwnd:
        raise ValueError(f"Window not found: {args.title!r}")
    original = W.POINT()
    check(get_pos(C.byref(original)), "GetCursorPos")
    previous_foreground = get_foreground()
    was_topmost = bool(get_window_style(hwnd, EXTENDED_WINDOW_STYLE_INDEX) & TOPMOST_STYLE)
    raised = False
    try:
        if args.foreground:
            check(set_window_pos(hwnd, W.HWND(HWND_TOPMOST), 0, 0, 0, 0, RESTORE_WINDOW_FLAGS), "SetWindowPos")
            raised = True
            check(foreground(hwnd), "SetForegroundWindow")
        for ordinal, target in enumerate(targets):
            expected = source_hashes(target, args.targets.resolve().parent)
            rect = W.RECT()
            check(get_client_rect(hwnd, C.byref(rect)), "GetClientRect")
            x, y = target["position"]
            if not (0 <= x < rect.right and 0 <= y < rect.bottom):
                raise ValueError(f"{target['name']}: position {target['position']} is outside the window client area")
            pt = W.POINT(x, y)
            check(client_to_screen(hwnd, C.byref(pt)), "ClientToScreen")
            # A nudge restores native Mouse identity after a BRP Custom Pointer releases control.
            check(set_pos(pt.x + POINTER_NUDGE, pt.y + POINTER_NUDGE), "SetCursorPos")
            time.sleep(NUDGE_SETTLE_SECONDS)
            check(set_pos(pt.x, pt.y), "SetCursorPos")
            time.sleep(HOVER_SETTLE_SECONDS)
            hit = window_from_point(pt)
            if hit != hwnd and not is_child(hwnd, hit):
                raise ValueError(f"{target['name']}: target position is covered by another window")
            seconds = sum(durations(target)) / MILLISECONDS_PER_SECOND * CAPTURE_ANIMATION_CYCLES + CAPTURE_MARGIN_SECONDS
            records, frames = collect(seconds)
            for record in records:
                record["matches_source_frame"] = record.get("sha256") in expected
                record["semantic"] = target["name"]
            save(args.output / folder(ordinal, target["name"]), records, frames)
            validate_capture(target, records)
            if any(r["position"] != [pt.x, pt.y] for r in records):
                raise ValueError(f"{target['name']}: OS mouse moved away from the target during capture")
    finally:
        active_error = sys.exception()
        cleanup_errors = []
        actions = [("restore mouse position", lambda: set_pos(original.x, original.y))]
        if raised:
            if is_window(hwnd):
                actions.append(("restore window topmost state", lambda: set_window_pos(
                    hwnd, W.HWND(HWND_TOPMOST if was_topmost else HWND_NOTOPMOST), 0, 0, 0, 0, RESTORE_WINDOW_FLAGS)))
            if previous_foreground and previous_foreground != hwnd and is_window(previous_foreground):
                actions.append(("restore foreground window", lambda: foreground(previous_foreground)))
        for operation, restore in actions:
            try:
                check(restore(), operation)
            except OSError as error:
                cleanup_errors.append(str(error))
        if cleanup_errors:
            message = "Native state cleanup failed: " + "; ".join(cleanup_errors)
            if active_error is not None:
                active_error.add_note(message)
            else:
                raise OSError(message)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--title", required=True, help="Exact title of the existing Gallery window")
    parser.add_argument("--targets", type=Path, required=True, help="UTF-8 target JSON with physical client positions")
    parser.add_argument("--foreground", action="store_true", help="Temporarily raise and focus the target window")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        capture_targets(args)
    except (OSError, ValueError) as error:
        parser.exit(1, f"{error}\n" + "".join(f"{note}\n" for note in getattr(error, "__notes__", [])))


if __name__ == "__main__":
    main()
