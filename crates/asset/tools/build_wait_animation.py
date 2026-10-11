import argparse
import hashlib
import json
import math
import shutil
from pathlib import Path

from PIL import Image, ImageDraw


MASTER = 320
CELL = 64
FRAMES = 24
FPS = 8
HOTSPOT = (25, 47)
CENTER = (HOTSPOT[0] * MASTER / CELL, HOTSPOT[1] * MASTER / CELL)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def layer(path):
    with Image.open(path) as image:
        if image.mode != 'RGBA' or image.size != (1254, 1254):
            raise ValueError(f'Expected 1254x1254 RGBA rig layer: {path}')
        return image.copy()


def scoop(index):
    if index < 4:
        return 0.0
    if index < 8:
        t = (index - 3) / 4
        return t * t * (3 - 2 * t)
    if index < 10:
        return 1.0
    if index < 14:
        t = (index - 9) / 4
        return 1 - t * t * (3 - 2 * t)
    return 0.0


def render(body, arm, empty_spoon, index):
    frame = Image.new('RGBA', (MASTER, MASTER))
    draw = ImageDraw.Draw(frame)
    cx, cy = CENTER
    draw.ellipse((cx - 64, cy - 64, cx + 64, cy + 64), fill='#454A9C')
    draw.ellipse((cx - 61, cy - 61, cx + 61, cy + 61), fill='#EEEFFF')
    draw.ellipse((cx - 41, cy - 41, cx + 41, cy + 41), fill=(0, 0, 0, 0))
    draw.ellipse((cx - 43, cy - 43, cx + 43, cy + 43), outline='#C6C8FA', width=2)
    colors = ['#164CF5', '#3C75FF', '#6898FF', '#98B7FF', '#BACFFD', '#D2DEFF', '#A5BFFB', '#5286FA']
    for sector, color in enumerate(colors):
        start = sector * 45 + index * 15 + 4
        draw.arc((cx - 52, cy - 52, cx + 52, cy + 52), start=start, end=start + 36, fill=color, width=14)
    frame.alpha_composite(body.resize((248, 248), Image.Resampling.LANCZOS), (44, 14))
    beat = scoop(index)
    angle = math.radians(31 - 14 * beat)
    scale = 1.4
    a, b = math.cos(angle) / scale, math.sin(angle) / scale
    d, e = -b, a
    pivot = (450, 803)
    destination = (470 + 10 * beat, 800)
    held = arm.copy()
    if 9 <= index < 18:
        held.alpha_composite(empty_spoon)
    moved = held.transform(held.size, Image.Transform.AFFINE,
        (a, b, pivot[0] - a * destination[0] - b * destination[1],
         d, e, pivot[1] - d * destination[0] - e * destination[1]), Image.Resampling.BICUBIC)
    frame.alpha_composite(moved.resize((248, 248), Image.Resampling.LANCZOS), (44, 14))
    return frame


def build(source, output, keyframes_only, runtime_dir):
    body, arm, empty_spoon = (layer(source / name) for name in ['body.png', 'arm.png', 'empty_spoon.png'])
    output.mkdir(parents=True, exist_ok=True)
    indices = [0, 4, 7, 10, 13] if keyframes_only else list(range(FRAMES))
    master_frames = [render(body, arm, empty_spoon, index) for index in indices]
    sheet = Image.new('RGB', (MASTER * len(indices), MASTER), '#E3E7F1')
    for position, frame in enumerate(master_frames):
        sheet.paste(frame, (MASTER * position, 0), frame)
    sheet.save(output / ('keyframes.png' if keyframes_only else 'contact-sheet.png'))
    if keyframes_only:
        return
    masters = output / 'frames_master_320'
    runtime = output / f'frames_runtime_{CELL}'
    masters.mkdir(exist_ok=True)
    runtime.mkdir(exist_ok=True)
    resized = []
    atlas = Image.new('RGBA', (CELL * 6, CELL * 4))
    for index, master in enumerate(master_frames):
        master.save(masters / f'wait_{index:02d}.png', optimize=True)
        item = master.resize((CELL, CELL), Image.Resampling.LANCZOS)
        item.save(runtime / f'wait_{index:02d}.png', optimize=True)
        resized.append(item)
        atlas.paste(item, (index % 6 * CELL, index // 6 * CELL))
    atlas_path = output / f'wait_atlas_{CELL}.png'
    atlas.save(atlas_path, optimize=True)
    resized[0].save(output / f'wait_static_{CELL}.png', optimize=True)
    master_frames[0].save(output / 'wait_preview.apng', save_all=True, append_images=master_frames[1:],
        duration=125, loop=0, disposal=0, blend=0)
    resized[0].save(output / 'wait_runtime_preview.apng', save_all=True, append_images=resized[1:],
        duration=125, loop=0, disposal=0, blend=0)
    for color, name in [('#E8EAF4', 'light'), ('#202336', 'dark')]:
        previews = []
        for frame in master_frames:
            preview = Image.new('RGBA', frame.size, color)
            preview.alpha_composite(frame)
            previews.append(preview.convert('RGB'))
        previews[0].save(output / f'wait_{name}.gif', save_all=True, append_images=previews[1:], duration=125, loop=0)
    marked = master_frames[0].copy()
    draw = ImageDraw.Draw(marked)
    cx, cy = CENTER
    draw.line((cx - 14, cy, cx + 14, cy), fill='#FF1665', width=2)
    draw.line((cx, cy - 14, cx, cy + 14), fill='#FF1665', width=2)
    marked.save(output / 'hotspot-check.png')
    with Image.open(atlas_path) as actual:
        for index, frame in enumerate(resized):
            x, y = index % 6 * CELL, index // 6 * CELL
            if actual.crop((x, y, x + CELL, y + CELL)).tobytes() != frame.tobytes():
                raise ValueError(f'Atlas crop mismatch: {index}')
    unique = len({frame.tobytes() for frame in resized})
    edges_clear = all(all(frame.getpixel((p, 0))[3] == 0 and frame.getpixel((p, CELL - 1))[3] == 0
        and frame.getpixel((0, p))[3] == 0 and frame.getpixel((CELL - 1, p))[3] == 0
        for p in range(CELL)) for frame in resized)
    if unique != FRAMES or not edges_clear:
        raise ValueError(f'Invalid frame QA: unique={unique}, transparent_edges={edges_clear}')
    report = {'cursor': 'Wait', 'frames': FRAMES, 'fps': FPS, 'loop': True,
        'master_frame_size': [MASTER, MASTER], 'runtime_frame_size': [CELL, CELL], 'grid': [6, 4],
        'hotspot': HOTSPOT, 'hotspot_validation': 'provisional', 'format_qc': 'passed',
        'art_qc': 'pending', 'animation_qc': 'pending', 'bevy_test': 'pending', 'native_mouse_test': 'pending',
        'unique_frames': unique, 'transparent_edges': edges_clear, 'atlas_crops_equal_sources': True,
        'sources': [{'path': p.name, 'sha256': digest(p)} for p in [source / name for name in ['body.png', 'arm.png', 'empty_spoon.png']]],
        'frames_sha256': [digest(runtime / f'wait_{i:02d}.png') for i in range(FRAMES)],
        'atlas_sha256': digest(atlas_path)}
    (output / 'wait-build-manifest.json').write_text(json.dumps(report, ensure_ascii=False, indent=2), encoding='utf-8')
    (output / 'wait.meta.toml').write_text(
        f'[cursor.Wait]\natlas = "wait_atlas_{CELL}.png"\nstatic_fallback = "wait_static_{CELL}.png"\n'
        f'frame_size = [{CELL}, {CELL}]\ncolumns = 6\nrows = 4\nframe_count = {FRAMES}\nfps = {FPS}\n'
        f'loop = true\nhotspot = [{HOTSPOT[0]}, {HOTSPOT[1]}]\nsource_master_frame_size = [320, 320]\n', encoding='utf-8')
    if runtime_dir is not None:
        static_path = runtime_dir / 'static_atlas_96.png'
        with Image.open(static_path) as existing:
            if existing.mode != 'RGBA' or existing.size != (768, 576):
                raise ValueError('Expected 768x576 RGBA static cursor atlas')
            static = existing.copy()
        static.paste(resized[0], (0, 384))
        static.save(static_path, optimize=True)
        shutil.copyfile(atlas_path, runtime_dir / atlas_path.name)
    print(json.dumps({'atlas': str(atlas_path), 'frames': unique, 'format_qc': 'passed'}))


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--source-dir', type=Path, default=Path(__file__).parent / 'wait')
    parser.add_argument('--output-dir', type=Path, required=True)
    parser.add_argument('--keyframes-only', action='store_true')
    parser.add_argument('--runtime-dir', type=Path)
    args = parser.parse_args()
    build(args.source_dir, args.output_dir, args.keyframes_only, args.runtime_dir)


if __name__ == '__main__':
    main()
