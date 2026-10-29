"""Validate theme-independent cursor atlases and make inspectable animated previews.

Requires Pillow and NumPy. Input is a JSON object with an `animations` array.
Paths in that array are relative to the manifest, never the current directory.
"""

import argparse
import hashlib
import html
import json
from pathlib import Path

import numpy as np
from PIL import Image


def positive(value, label):
    if not isinstance(value, int) or isinstance(value, bool) or value <= 0:
        raise ValueError(f"{label} must be a positive integer")
    return value


def process(manifest_path, output):
    manifest_path, output = Path(manifest_path).resolve(), Path(output).resolve()
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    items = manifest["animations"]
    if not items:
        raise ValueError("At least one animation is required")
    output.mkdir(parents=True, exist_ok=True)
    reports, cards, names = [], [], set()
    for number, item in enumerate(items):
        name = item["name"]
        if not isinstance(name, str) or not name or name in names:
            raise ValueError("Animation names must be unique nonempty strings")
        names.add(name)
        count = positive(item["frames"], f"{name}: frames")
        if count < 2:
            raise ValueError(f"{name}: a loop needs more than one frame")
        width, height = [positive(v, f"{name}: frame dimension") for v in item["frame_size"]]
        cols = positive(item["columns"], f"{name}: columns")
        hotspot = item["hotspot"]
        if len(hotspot) != 2 or any(not isinstance(v, int) or isinstance(v, bool) for v in hotspot) or not (0 <= hotspot[0] < width and 0 <= hotspot[1] < height):
            raise ValueError(f"{name}: hotspot lies outside its frame")
        durations = item.get("durations_ms")
        if durations is None:
            durations = [positive(item["frame_duration_ms"], f"{name}: duration")] * count
        if len(durations) != count:
            raise ValueError(f"{name}: duration count differs from frame count")
        durations = [positive(d, f"{name}: duration") for d in durations]
        atlas_path = manifest_path.parent / item["atlas"]
        with Image.open(atlas_path) as source:
            if source.mode != "RGBA" or source.size != (cols * width, ((count + cols - 1) // cols) * height):
                raise ValueError(f"{name}: expected exact RGBA atlas dimensions")
            atlas = source.copy()
        frames, arrays = [], []
        anchor = item.get("anchor_rect")
        if anchor is not None and (len(anchor) != 4 or any(not isinstance(v, int) for v in anchor) or not (0 <= anchor[0] < anchor[2] <= width and 0 <= anchor[1] < anchor[3] <= height)):
            raise ValueError(f"{name}: invalid protected anchor rectangle")
        for index in range(count):
            x, y = index % cols * width, index // cols * height
            frame = atlas.crop((x, y, x + width, y + height))
            pixels = np.array(frame)
            if not np.any(pixels[:, :, 3]):
                raise ValueError(f"{name}: empty frame {index}")
            if np.any(pixels[0, :, 3]) or np.any(pixels[-1, :, 3]) or np.any(pixels[:, 0, 3]) or np.any(pixels[:, -1, 3]):
                raise ValueError(f"{name}: frame {index} reaches the canvas boundary")
            if np.any(pixels[pixels[:, :, 3] == 0, :3]):
                raise ValueError(f"{name}: hidden RGB in transparent pixels of frame {index}")
            frames.append(frame)
            # Premultiply before difference metrics so nearly transparent RGB does not dominate.
            rgba = pixels.astype(np.float32) / 255
            rgba[:, :, :3] *= rgba[:, :, 3:4]
            arrays.append(rgba)
        unique = len({f.tobytes() for f in frames})
        if unique < 2:
            raise ValueError(f"{name}: all frames are identical")
        if anchor and len({f.crop(anchor).tobytes() for f in frames}) != 1:
            raise ValueError(f"{name}: the protected anchor changes between frames")
        deltas = [float(np.mean(np.abs(a - b))) for a, b in zip(arrays, arrays[1:])]
        seam = float(np.mean(np.abs(arrays[-1] - arrays[0])))
        p95 = float(np.percentile(deltas, 95))
        seam_ratio = seam / max(p95, 1e-10)
        filename = f"cursor-{number:03d}.png"
        frames[0].save(output / filename, format="PNG", save_all=True, append_images=frames[1:], duration=durations, loop=0, disposal=0, blend=0)
        with Image.open(output / filename) as exported:
            if not exported.is_animated or exported.info.get("loop") != 0:
                raise ValueError(f"{name}: preview export is not a continuous loop")
            if abs(sum(exported.seek(i) or exported.info["duration"] for i in range(exported.n_frames)) - sum(durations)) > 1:
                raise ValueError(f"{name}: exported timing differs")
        strip = Image.new("RGBA", (width * 8, height), "#252A40")
        for k in range(8):
            strip.alpha_composite(frames[k * count // 8], (k * width, 0))
        strip.convert("RGB").save(output / f"motion-{number:03d}.png")
        reports.append({"name": name, "technical": "passed", "frame_size": [width, height], "frames": count, "unique_frames": unique, "loop_ms": sum(durations), "hotspot": hotspot, "anchor_checked": anchor is not None, "seam_to_p95": seam_ratio, "seam_needs_attention": seam_ratio > 3, "atlas_sha256": hashlib.sha256(atlas_path.read_bytes()).hexdigest(), "visual_review": "required", "native_runtime": "not_checked_by_this_tool"})
        cards.append(f'<article><img src="{filename}" width="{width}" height="{height}"><div><b>{html.escape(name)}</b><small>{count} frames · {sum(durations)}ms · hotspot {hotspot}</small><p>{html.escape(item.get("motion", ""))}</p><a href="motion-{number:03d}.png">Motion strip</a></div></article>')
    (output / "qc.json").write_text(json.dumps(reports, indent=2), encoding="utf-8")
    (output / "index.html").write_text('''<!doctype html><meta charset="utf-8"><title>Animated cursors</title><style>body{background:#181b2a;color:#efeaff;font:15px system-ui;margin:24px}main{display:grid;grid-template-columns:repeat(auto-fit,minmax(330px,1fr));gap:12px}article{display:flex;align-items:center;gap:18px;padding:16px;background:#292e43;border-radius:10px}img{image-rendering:auto;flex-shrink:0;background:repeating-conic-gradient(#33394d 0 25%,#2c3144 0 50%) 0/16px 16px}small{display:block;color:#b7caff}p{font-size:12px;color:#d4cbef}a{color:#a2dbff}button{padding:8px;margin:8px}</style><h1>Animated cursors</h1><p>Native frame size · actual independent timing · continuous APNG loops</p><button onclick="document.querySelectorAll('img').forEach(i=>i.style.background='#e8eaf4')">Light</button><button onclick="document.querySelectorAll('img').forEach(i=>i.style.background='#202336')">Dark</button><main>''' + "\n".join(cards) + "</main>", encoding="utf-8")
    return reports


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("manifest", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    report = process(args.manifest, args.output)
    print(json.dumps({"count": len(report), "technical": "passed", "seams_to_inspect": [r["name"] for r in report if r["seam_needs_attention"]]}))
