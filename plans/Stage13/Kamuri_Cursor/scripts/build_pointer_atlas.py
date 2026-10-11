#!/usr/bin/env python3
"""Build a Bevy-ready Pointer PNG atlas from 12 RGBA source frames.

Only normalizes raster dimensions and packaging, never generates character motion.
"""
import argparse
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageDraw


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(folder, cell=128, columns=4, hotspot=(44, 108)):
    src = folder / 'frames_master_320'
    paths = [src / f'pointer_{i:02d}.png' for i in range(12)]
    missing = [str(p) for p in paths if not p.is_file()]
    if missing:
        raise FileNotFoundError('Missing: ' + ', '.join(missing))
    frames = []
    for p in paths:
        with Image.open(p) as item:
            if item.mode != 'RGBA' or item.size != (320, 320):
                raise ValueError(f'Expected 320x320 RGBA: {p}; got {item.mode} {item.size}')
            if item.getpixel((0, 0))[3] != 0:
                raise ValueError(f'Background is not transparent: {p}')
            frames.append(item.copy())
    if not (0 <= hotspot[0] < cell and 0 <= hotspot[1] < cell):
        raise ValueError('hotspot must be in frame bounds')
    runtime = folder / f'frames_runtime_{cell}'
    runtime.mkdir(parents=True, exist_ok=True)
    resized = []
    for idx, image in enumerate(frames):
        dst = image.resize((cell, cell), Image.Resampling.LANCZOS)
        dst.save(runtime / f'pointer_{idx:02d}.png', optimize=True)
        resized.append(dst)
    atlas = Image.new('RGBA', (cell * columns, cell * 3), (0, 0, 0, 0))
    for index, image in enumerate(resized):
        atlas.paste(image, ((index % columns) * cell, (index // columns) * cell))
    atlas_out = folder / f'pointer_atlas_{cell}.png'
    atlas.save(atlas_out, optimize=True)
    resized[0].save(folder / f'pointer_static_{cell}.png', optimize=True)
    master = Image.new('RGBA', (320 * columns, 320 * 3), (0, 0, 0, 0))
    for index, image in enumerate(frames):
        master.paste(image, ((index % columns) * 320, (index // columns) * 320))
    master.save(folder / 'pointer_atlas_master_320.png', optimize=True)
    with Image.open(atlas_out) as actual:
        for index, expected in enumerate(resized):
            x = (index % columns) * cell
            y = (index // columns) * cell
            if actual.crop((x, y, x + cell, y + cell)).tobytes() != expected.tobytes():
                raise RuntimeError(f'Atlas pixel mismatch for frame {index}')
    marked = resized[5].copy()
    draw = ImageDraw.Draw(marked)
    x, y = hotspot
    draw.ellipse((x-4, y-4, x+4, y+4), outline='#ff1665', width=2)
    draw.line((x-11, y, x+11, y), fill='#ff1665', width=1)
    draw.line((x, y-11, x, y+11), fill='#ff1665', width=1)
    (folder / 'previews').mkdir(exist_ok=True)
    marked.save(folder / 'previews/pointer_hotspot_check.png')
    config = f'''# Custom delivery format; not a Bevy built-in schema.
# Hotspot is provisional. Confirm with a real mouse on Windows.
[cursor.Pointer]
atlas = "pointer_atlas_{cell}.png"
static_fallback = "pointer_static_{cell}.png"
frame_size = [{cell}, {cell}]
columns = {columns}
rows = 3
frame_count = 12
fps = 8
loop = true
hotspot = [{x}, {y}]
source_master_frame_size = [320, 320]
'''
    (folder / 'pointer.meta.toml').write_text(config, encoding='utf-8')
    manifest = {
        'frames': 12, 'fps': 8, 'loop': True,
        'runtime_size': [cell, cell], 'hotspot': [x, y],
        'hotspot_status': 'provisional; manual Windows mouse test required',
        'alpha': 'RGBA',
        'source_frames': [{'path': str(p.relative_to(folder)), 'sha256': digest(p)} for p in paths],
        'runtime_frames': [{'path': str((runtime / f'pointer_{i:02d}.png').relative_to(folder)),
                            'sha256': digest(runtime / f'pointer_{i:02d}.png')} for i in range(12)],
        'atlas': {'path': atlas_out.name, 'sha256': digest(atlas_out), 'grid': [4, 3]},
    }
    (folder / 'pointer-build-manifest.json').write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False), encoding='utf-8'
    )
    print(f'Generated {atlas_out}: {atlas.size}, RGBA, hotspot={hotspot}, verified 12 frames')


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--pointer-dir', required=True, type=Path)
    p.add_argument('--size', type=int, default=128)
    p.add_argument('--hotspot-x', type=int, default=44)
    p.add_argument('--hotspot-y', type=int, default=108)
    args = p.parse_args()
    build(args.pointer_dir, args.size, 4, (args.hotspot_x, args.hotspot_y))


if __name__ == '__main__':
    main()
