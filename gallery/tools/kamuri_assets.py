import argparse
import base64
import hashlib
import json
from pathlib import Path

from PIL import Image, ImageChops


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1] / "src/assets/splash/kamuri"
    layout = json.loads((root / "layer_map.json").read_text(encoding="utf-8"))
    images = {}
    for layer in layout["layers"]:
        path = root / layer["file"]
        assert hashlib.sha256(path.read_bytes()).hexdigest() == layer["sha256"]
        im = Image.open(path).convert("RGBA")
        assert list(im.size) == layer["source_size"]
        assert list(im.getchannel("A").point(lambda a: 255 if a >= 16 else 0).getbbox()) == layer["alpha_bbox"]
        assert layer["scale"] > 0
        assert layer["parent"] == "character"
        images[layer["name"]] = im
    diff = ImageChops.difference(images["body_open"], images["body_closed"])
    channels = diff.split()
    mask = channels[0]
    for channel in channels[1:]:
        mask = ImageChops.lighter(mask, channel)
    changed = sum(value != 0 for value in mask.tobytes())
    assert mask.getbbox() == (662, 378, 1022, 616)
    assert changed == 55209
    upper, lower = [layer for layer in layout["layers"] if layer["name"].startswith("ribbon")]
    assert upper["pivot"] == lower["pivot"]
    assert layout["canvas"]["alpha"] == 1
    patch = next(layer for layer in layout["layers"] if layer["name"] == "sleeve_patch")
    assert patch["source_rect"] == [990, 800, 1672, 941]
    assert patch["draw_size"] == [682, 141]
    assert patch["pivot"] == [990, 800]
    skin = lambda c: c[3] > 240 and c[0] > 220 and c[0] > c[1] + 15 and c[1] > c[2] + 7
    points = [(x, y) for y in range(885, 924) for x in range(1060, 1190) if skin(images["body_open"].getpixel((x, y)))]
    assert len(points) == 1938
    assert all(images["sleeve_patch"].getpixel(point)[3] >= 190 and not skin(images["sleeve_patch"].getpixel(point)) for point in points)
    arm = next(layer for layer in layout["layers"] if layer["name"] == "arm")
    assert arm["pivot"] == [1250, 735]
    args.output.mkdir(parents=True, exist_ok=True)
    report = {"blink_changed_pixels": changed, "blink_bbox": list(mask.getbbox()), "sha256_and_bbox": "passed", "shared_ribbon_pivot": upper["pivot"], "shoulder_pivot": arm["pivot"], "corrected_hand_pixels": len(points)}
    (args.output / "asset-check.json").write_text(json.dumps(report, indent=2), encoding="utf-8")
    payload = []
    for layer in layout["layers"]:
        payload.append(dict(layer, url="data:image/png;base64," + base64.b64encode((root / layer["file"]).read_bytes()).decode()))
    template = (Path(__file__).with_name("kamuri_preview.html")).read_text(encoding="utf-8")
    template = template.replace("LAYERS_JSON", json.dumps(payload)).replace("LAYOUT_JSON", json.dumps(layout))
    (args.output / "preview.html").write_text(template, encoding="utf-8")
    print(json.dumps(report))


if __name__ == "__main__":
    main()
