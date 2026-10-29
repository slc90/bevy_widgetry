import json
import math
from pathlib import Path
import tempfile
import unittest

from PIL import Image, ImageDraw

from cursor_tool import process


def make_study(output):
    output = Path(output)
    output.mkdir(parents=True, exist_ok=True)
    specs = [
        dict(name="Inspect", atlas="inspect.png", frame_size=[48, 48], columns=4,
             frames=16, frame_duration_ms=180, hotspot=[10, 10], anchor_rect=[9, 9, 12, 12]),
        dict(name="Orbit", atlas="orbit.png", frame_size=[48, 48], columns=4,
             frames=24, durations_ms=[160, 200, 240] * 8, hotspot=[24, 24], anchor_rect=[23, 23, 26, 26]),
    ]
    for spec in specs:
        atlas = Image.new("RGBA", (192, ((spec["frames"] + 3) // 4) * 48))
        for i in range(spec["frames"]):
            frame = Image.new("RGBA", (48, 48))
            draw = ImageDraw.Draw(frame)
            if spec["name"] == "Inspect":
                draw.line((4, 10, 16, 10), fill="#F7FFF9", width=3)
                draw.line((10, 4, 10, 16), fill="#F7FFF9", width=3)
                x = 22 + 7 * (1 - math.cos(math.tau * i / spec["frames"]))
                draw.rounded_rectangle((x, 22, x + 4, 36), radius=2, fill="#70DDC8")
            else:
                draw.ellipse((12, 12, 36, 36), outline="#70DDC8", width=2)
                x, y = 24 + 12 * math.cos(math.tau * i / spec["frames"]), 24 + 12 * math.sin(math.tau * i / spec["frames"])
                draw.ellipse((x - 3, y - 3, x + 3, y + 3), fill="#F7FFF9")
                draw.ellipse((22, 22, 26, 26), fill="#70DDC8")
            atlas.paste(frame, (i % 4 * 48, i // 4 * 48))
        atlas.save(output / spec["atlas"])
    (output / "manifest.json").write_text(json.dumps({"animations": specs}), encoding="utf-8")
    return output / "manifest.json"


class CursorToolTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.manifest = make_study(self.root)

    def rewrite(self, change):
        data = json.loads(self.manifest.read_text())
        change(data["animations"][0])
        self.manifest.write_text(json.dumps(data), encoding="utf-8")

    def test_exports_different_semantics_sizes_and_variable_timing(self):
        report = process(self.manifest, self.root / "preview")
        self.assertEqual([r["frame_size"] for r in report], [[48, 48], [48, 48]])
        self.assertEqual([r["loop_ms"] for r in report], [2880, 4800])
        self.assertTrue(all(r["anchor_checked"] and r["unique_frames"] > 3 for r in report))
        self.assertTrue((self.root / "preview/index.html").is_file())

    def test_rejects_out_of_frame_hotspot(self):
        self.rewrite(lambda s: s.update(hotspot=[48, 10]))
        with self.assertRaisesRegex(ValueError, "hotspot"):
            process(self.manifest, self.root / "preview")

    def test_two_frame_loop_remains_a_valid_animation(self):
        data = json.loads(self.manifest.read_text())
        item = data["animations"][0]
        with Image.open(self.root / item["atlas"]) as source:
            atlas = Image.new("RGBA", (96, 48))
            atlas.paste(source.crop((0, 0, 48, 48)), (0, 0))
            atlas.paste(source.crop((0, 96, 48, 144)), (48, 0))
            atlas.save(self.root / "two.png")
        item.update(atlas="two.png", columns=2, frames=2, frame_duration_ms=300)
        self.manifest.write_text(json.dumps({"animations": [item]}), encoding="utf-8")
        report = process(self.manifest, self.root / "preview")
        self.assertEqual(report[0]["unique_frames"], 2)
        self.assertEqual(report[0]["loop_ms"], 600)

    def test_rejects_duration_count_mismatch(self):
        self.rewrite(lambda s: s.update(durations_ms=[80]))
        with self.assertRaisesRegex(ValueError, "duration count"):
            process(self.manifest, self.root / "preview")

    def test_rejects_anchor_drift(self):
        path = self.root / "inspect.png"
        with Image.open(path) as image:
            image.putpixel((58, 10), (255, 0, 0, 255))
            image.save(path)
        with self.assertRaisesRegex(ValueError, "anchor changes"):
            process(self.manifest, self.root / "preview")

    def test_rejects_clipped_edges(self):
        path = self.root / "inspect.png"
        with Image.open(path) as image:
            image.putpixel((0, 0), (255, 0, 0, 255))
            image.save(path)
        with self.assertRaisesRegex(ValueError, "canvas boundary"):
            process(self.manifest, self.root / "preview")

    def test_rejects_nonanimated_atlas(self):
        path = self.root / "inspect.png"
        with Image.open(path) as image:
            frame = image.crop((0, 0, 48, 48))
            for i in range(16):
                image.paste(frame, (i % 4 * 48, i // 4 * 48))
            image.save(path)
        with self.assertRaisesRegex(ValueError, "all frames are identical"):
            process(self.manifest, self.root / "preview")


if __name__ == "__main__":
    import sys
    if len(sys.argv) == 3 and sys.argv[1] == "--write-study":
        make_study(sys.argv[2])
    else:
        unittest.main()
