#!/usr/bin/env python3
"""Build a Bevy-ready Pointer PNG atlas from 12 RGBA source frames.

Only normalizes raster dimensions and packaging, never generates character motion.
"""
import argparse
import hashlib
import json
from pathlib import Path
from PIL import Image, ImageDraw


DEFAULT_CELL_SIZE = 128
DEFAULT_COLUMNS = 4
DEFAULT_HOTSPOT = (44, 108)
FRAME_COUNT = 12
ATLAS_ROWS = 3
SOURCE_FRAME_SIZE = 320
ANIMATION_FPS = 8
HOTSPOT_PREVIEW_FRAME = 5
HOTSPOT_RING_RADIUS = 4
HOTSPOT_CROSS_HALF_SIZE = 11

def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build(folder, cell=DEFAULT_CELL_SIZE, columns=DEFAULT_COLUMNS, hotspot=DEFAULT_HOTSPOT):
    src = folder / 'frames_master_320'
    paths = [src / f'pointer_{i:02d}.png' for i in range(FRAME_COUNT)]
    missing = [str(p) for p in paths if not p.is_file()]
    if missing:
        raise FileNotFoundError('Missing: ' + ', '.join(missing))
    frames = []
    for p in paths:
        with Image.open(p) as item:
            if item.mode != 'RGBA' or item.size != (SOURCE_FRAME_SIZE, SOURCE_FRAME_SIZE):
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
    atlas = Image.new('RGBA', (cell * columns, cell * ATLAS_ROWS), (0, 0, 0, 0))
    for index, image in enumerate(resized):
        atlas.paste(image, ((index % columns) * cell, (index // columns) * cell))
    atlas_out = folder / f'pointer_atlas_{cell}.png'
    atlas.save(atlas_out, optimize=True)
    resized[0].save(folder / f'pointer_static_{cell}.png', optimize=True)
    master = Image.new('RGBA', (SOURCE_FRAME_SIZE * columns, SOURCE_FRAME_SIZE * ATLAS_ROWS), (0, 0, 0, 0))
    for index, image in enumerate(frames):
        master.paste(image, ((index % columns) * SOURCE_FRAME_SIZE, (index // columns) * SOURCE_FRAME_SIZE))
    master.save(folder / 'pointer_atlas_master_320.png', optimize=True)
    with Image.open(atlas_out) as actual:
        for index, expected in enumerate(resized):
            x = (index % columns) * cell
            y = (index // columns) * cell
            if actual.crop((x, y, x + cell, y + cell)).tobytes() != expected.tobytes():
                raise RuntimeError(f'Atlas pixel mismatch for frame {index}')
    marked = resized[HOTSPOT_PREVIEW_FRAME].copy()
    draw = ImageDraw.Draw(marked)
    x, y = hotspot
    draw.ellipse((x-HOTSPOT_RING_RADIUS, y-HOTSPOT_RING_RADIUS, x+HOTSPOT_RING_RADIUS, y+HOTSPOT_RING_RADIUS), outline='#ff1665', width=2)
    draw.line((x-HOTSPOT_CROSS_HALF_SIZE, y, x+HOTSPOT_CROSS_HALF_SIZE, y), fill='#ff1665', width=1)
    draw.line((x, y-HOTSPOT_CROSS_HALF_SIZE, x, y+HOTSPOT_CROSS_HALF_SIZE), fill='#ff1665', width=1)
    (folder / 'previews').mkdir(exist_ok=True)
    marked.save(folder / 'previews/pointer_hotspot_check.png')
    config = f'''# Custom delivery format; not a Bevy built-in schema.
# Hotspot is provisional. Confirm with a real mouse on Windows.
[cursor.Pointer]
atlas = "pointer_atlas_{cell}.png"
static_fallback = "pointer_static_{cell}.png"
frame_size = [{cell}, {cell}]
columns = {columns}
rows = {ATLAS_ROWS}
frame_count = {FRAME_COUNT}
fps = {ANIMATION_FPS}
loop = true
hotspot = [{x}, {y}]
source_master_frame_size = [{SOURCE_FRAME_SIZE}, {SOURCE_FRAME_SIZE}]
'''
    (folder / 'pointer.meta.toml').write_text(config, encoding='utf-8')
    manifest = {
        'frames': FRAME_COUNT, 'fps': ANIMATION_FPS, 'loop': True,
        'runtime_size': [cell, cell], 'hotspot': [x, y],
        'hotspot_status': 'provisional; manual Windows mouse test required',
        'alpha': 'RGBA',
        'source_frames': [{'path': str(p.relative_to(folder)), 'sha256': digest(p)} for p in paths],
        'runtime_frames': [{'path': str((runtime / f'pointer_{i:02d}.png').relative_to(folder)),
                            'sha256': digest(runtime / f'pointer_{i:02d}.png')} for i in range(FRAME_COUNT)],
        'atlas': {'path': atlas_out.name, 'sha256': digest(atlas_out), 'grid': [DEFAULT_COLUMNS, ATLAS_ROWS]},
    }
    (folder / 'pointer-build-manifest.json').write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False), encoding='utf-8'
    )
    print(f'Generated {atlas_out}: {atlas.size}, RGBA, hotspot={hotspot}, verified 12 frames')


def main():
    p = argparse.ArgumentParser()
    p.add_argument('--pointer-dir', required=True, type=Path)
    p.add_argument('--size', type=int, default=DEFAULT_CELL_SIZE)
    p.add_argument('--hotspot-x', type=int, default=DEFAULT_HOTSPOT[0])
    p.add_argument('--hotspot-y', type=int, default=DEFAULT_HOTSPOT[1])
    args = p.parse_args()
    build(args.pointer_dir, args.size, DEFAULT_COLUMNS, (args.hotspot_x, args.hotspot_y))


if __name__ == '__main__':
    main()
