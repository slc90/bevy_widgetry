import argparse
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageChops


SLOT = (384, 480, 480, 576)
HOTSPOT = (14, 37)


def build(source, output, atlas_path):
    with Image.open(source) as image:
        if image.mode != 'RGBA':
            raise ValueError('ReadOnly source must contain an RGBA alpha channel')
        bounds = image.getchannel('A').getbbox()
        if bounds is None:
            raise ValueError('ReadOnly source is empty')
        sprite = image.crop(bounds)
        sprite.thumbnail((46, 46), Image.Resampling.LANCZOS)
    tile = Image.new('RGBA', (96, 96))
    offset = ((48 - sprite.width) // 2, (48 - sprite.height) // 2)
    tile.paste(sprite, offset)
    output.mkdir(parents=True, exist_ok=True)
    tile.crop((0, 0, 48, 48)).save(output / 'readonly_48.png')
    report = {
        'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
        'source_bounds': bounds,
        'visible_size': sprite.size,
        'offset': offset,
        'index': 44,
        'slot': SLOT,
        'hotspot': HOTSPOT,
    }
    if atlas_path is not None:
        with Image.open(atlas_path) as image:
            if image.mode != 'RGBA' or image.size != (768, 576):
                raise ValueError('Expected the existing 768x576 RGBA static atlas')
            original = image.copy()
        updated = original.copy()
        updated.paste(tile, SLOT[:2])
        difference = ImageChops.difference(original, updated)
        difference.paste((0, 0, 0, 0), SLOT)
        if any(channel.getbbox() for channel in difference.split()):
            raise ValueError('Atlas packing changed pixels outside slot 44')
        updated.save(atlas_path)
        report['other_slots_unchanged'] = True
    (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--source', type=Path, default=Path(__file__).parent / 'readonly/source.png')
    parser.add_argument('--output-dir', type=Path, default=Path('target/cursor/readonly'))
    parser.add_argument('--runtime-atlas', type=Path)
    args = parser.parse_args()
    build(args.source, args.output_dir, args.runtime_atlas)
