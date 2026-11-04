import ctypes as C
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

from PIL import Image

import native_cursor_probe as probe


class NativeCursorProbeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.directory = Path(self.temp.name)
        atlas = Image.new("RGBA", (4, 2), (100, 51, 21, 128))
        atlas.paste((255, 255, 255, 255), (2, 0, 4, 2))
        atlas.save(self.directory / "冠.png")
        self.target = dict(name="冠", atlas="冠.png", frame_size=[2, 2], columns=2,
                           frames=2, durations_ms=[70, 90], hotspot=[1, 0], position=[20, 30])
        self.path = self.directory / "targets.json"
        self.write_targets([self.target])

    def write_targets(self, targets):
        self.path.write_text(json.dumps(targets, ensure_ascii=False), encoding="utf-8")

    def test_unicode_targets_use_utf8_and_relative_atlas_paths(self):
        original = Path.read_text

        def legacy_default(path, *args, **kwargs):
            kwargs.setdefault("encoding", "cp936")
            return original(path, *args, **kwargs)

        with patch.object(Path, "read_text", autospec=True, side_effect=legacy_default):
            targets = probe.load_targets(self.path)
        self.assertEqual(targets, [self.target])
        hashes = probe.source_hashes(targets[0], self.path.parent)
        for pixel in (bytes([21, 51, 100, 128]), bytes([10, 25, 50, 128]),
                      bytes([11, 26, 50, 128]), bytes([255, 255, 255, 255])):
            self.assertIn(hashlib.sha256(pixel * 4).hexdigest(), hashes)

    def test_invalid_targets_fail_before_any_native_interaction(self):
        variants = [[], {}, [dict(self.target, position=[-1, 30])],
                    [dict(self.target, durations_ms=[0, 90])],
                    [dict(self.target, hotspot=[2, 0])],
                    [dict(self.target, frames=3, durations_ms=[1, 1, 1])],
                    [dict(self.target, atlas="missing.png")]]
        for targets in variants:
            with self.subTest(targets=targets), patch.object(probe, "find_window") as find:
                self.write_targets(targets)
                with self.assertRaises((ValueError, OSError)):
                    probe.capture_targets(SimpleNamespace(title="Gallery", targets=self.path,
                                                          foreground=True, output=self.directory))
                find.assert_not_called()

    def test_capture_validation_rejects_each_observable_failure(self):
        good = [dict(visible=True, size=[2, 2], hotspot=[1, 0], sha256=str(i),
                     matches_source_frame=True) for i in range(2)]
        variants = [([], "No native samples"),
                    ([dict(r, visible=False) for r in good], "not visible"),
                    ([dict(r, size=[3, 2]) for r in good], "size"),
                    ([dict(r, hotspot=[0, 0]) for r in good], "hotspot"),
                    ([dict(r, sha256="same") for r in good], "did not animate"),
                    ([dict(r, matches_source_frame=False) for r in good], "atlas")]
        probe.validate_capture(self.target, good)
        for records, message in variants:
            with self.subTest(message=message), self.assertRaisesRegex(ValueError, message):
                probe.validate_capture(self.target, records)

    def native_patches(self, *, fail=False, topmost=False):
        hashes = sorted(probe.source_hashes(self.target, self.directory))
        records = [dict(visible=True, size=[2, 2], hotspot=[1, 0], sha256=h,
                        handle=3, elapsed=i * .025, position=[20, 30])
                   for i, h in enumerate(hashes[:2])]

        def get_pos(point):
            point._obj.x, point._obj.y = 7, 9
            return True

        def client_rect(hwnd, rect):
            rect._obj.right, rect._obj.bottom = 100, 100
            return True

        mocks = dict(find_window=Mock(return_value=10), get_pos=Mock(side_effect=get_pos),
                     get_foreground=Mock(return_value=20), is_window=Mock(return_value=True),
                     get_window_style=Mock(return_value=8 if topmost else 0),
                     set_window_pos=Mock(return_value=True), foreground=Mock(return_value=True),
                     set_pos=Mock(return_value=True), get_client_rect=Mock(side_effect=client_rect),
                     client_to_screen=Mock(return_value=True), window_from_point=Mock(return_value=10),
                     is_child=Mock(return_value=False), save=Mock(),
                     collect=Mock(side_effect=RuntimeError("capture failed")) if fail else
                     Mock(return_value=(records, [])))
        return mocks

    def test_success_and_failure_restore_mouse_foreground_and_topmost(self):
        for fail in (False, True):
            for was_topmost in (False, True):
                with self.subTest(fail=fail, was_topmost=was_topmost):
                    mocks = self.native_patches(fail=fail, topmost=was_topmost)
                    args = SimpleNamespace(title="Gallery", targets=self.path, foreground=True,
                                           output=self.directory / "output")
                    with patch.multiple(probe, **mocks), patch.object(probe.time, "sleep"):
                        if fail:
                            with self.assertRaisesRegex(RuntimeError, "capture failed"):
                                probe.capture_targets(args)
                        else:
                            probe.capture_targets(args)
                    self.assertEqual(mocks["set_pos"].call_args.args, (7, 9))
                    self.assertEqual(mocks["foreground"].call_args.args, (20,))
                    restored = mocks["set_window_pos"].call_args.args[1].value
                    self.assertEqual(restored, C.c_void_p(-1 if was_topmost else -2).value)

    def test_invalid_client_position_restores_temporary_window_state(self):
        mocks = self.native_patches()
        self.target["position"] = [100, 30]
        self.write_targets([self.target])
        with patch.multiple(probe, **mocks):
            with self.assertRaisesRegex(ValueError, "outside.*client"):
                probe.capture_targets(SimpleNamespace(title="Gallery", targets=self.path,
                                                      foreground=True, output=self.directory))
        mocks["collect"].assert_not_called()
        self.assertEqual(mocks["foreground"].call_args.args, (20,))

    def test_missing_window_fails_without_changing_native_state(self):
        with patch.object(probe, "find_window", return_value=None), patch.object(probe, "get_pos") as get:
            with self.assertRaisesRegex(ValueError, "Window not found"):
                probe.capture_targets(SimpleNamespace(title="Missing", targets=self.path,
                                                      foreground=True, output=self.directory))
            get.assert_not_called()

    def test_capture_without_foreground_does_not_change_window_order_or_focus(self):
        mocks = self.native_patches()
        with patch.multiple(probe, **mocks), patch.object(probe.time, "sleep"):
            probe.capture_targets(SimpleNamespace(title="Gallery", targets=self.path,
                                                  foreground=False, output=self.directory))
        mocks["set_window_pos"].assert_not_called()
        mocks["foreground"].assert_not_called()
        self.assertEqual(mocks["set_pos"].call_args.args, (7, 9))

    def test_covered_target_fails_before_capture_and_restores_state(self):
        mocks = self.native_patches()
        mocks["window_from_point"].return_value = 99
        with patch.multiple(probe, **mocks), patch.object(probe.time, "sleep"):
            with self.assertRaisesRegex(ValueError, "covered by another window"):
                probe.capture_targets(SimpleNamespace(title="Gallery", targets=self.path,
                                                      foreground=True, output=self.directory))
        mocks["collect"].assert_not_called()
        self.assertEqual(mocks["foreground"].call_args.args, (20,))
        self.assertEqual(mocks["set_pos"].call_args.args, (7, 9))

    def test_cleanup_failure_keeps_original_error_and_attempts_remaining_restoration(self):
        mocks = self.native_patches(fail=True)
        mocks["set_pos"].side_effect = [True, True, False]
        with patch.multiple(probe, **mocks), patch.object(probe.time, "sleep"):
            with self.assertRaisesRegex(RuntimeError, "capture failed") as raised:
                probe.capture_targets(SimpleNamespace(title="Gallery", targets=self.path,
                                                      foreground=True, output=self.directory))
        self.assertIn("restore mouse position", raised.exception.__notes__[0])
        self.assertEqual(mocks["foreground"].call_args.args, (20,))

    def test_snapshot_releases_dc_and_both_bitmaps_when_bitmap_read_fails(self):
        def cursor(info):
            info._obj.hCursor = 3
            return True

        def icon(handle, info):
            info._obj.hbmColor, info._obj.hbmMask = 4, 5
            return True

        def bitmap(handle, size, info):
            info._obj.width, info._obj.height = 2, 2
            return True

        with patch.multiple(probe, get_cursor=Mock(side_effect=cursor),
                            get_icon=Mock(side_effect=icon), get_object=Mock(side_effect=bitmap),
                            get_dc=Mock(return_value=6), get_dib=Mock(return_value=0),
                            release_dc=Mock(return_value=1), delete_object=Mock(return_value=1)):
            with self.assertRaises(OSError):
                probe.snapshot()
            probe.release_dc.assert_called_once_with(None, 6)
            self.assertEqual([call.args for call in probe.delete_object.call_args_list], [(4,), (5,)])

    def test_saved_evidence_keeps_sample_times_and_readable_previews(self):
        records = [dict(elapsed=.03, handle=1, sha256="a", size=[2, 2], hotspot=[1, 0], visible=True),
                   dict(elapsed=.10, handle=2, sha256="b", size=[2, 2], hotspot=[1, 0], visible=True)]
        frames = [(Image.new("RGBA", (2, 2), color), record["elapsed"])
                  for color, record in zip(("red", "blue"), records)]
        probe.save(self.directory / "evidence", records, frames)
        output = self.directory / "evidence"
        self.assertEqual(json.loads((output / "native.json").read_text(encoding="utf-8")), records)
        with Image.open(output / "native.apng") as preview:
            self.assertEqual(preview.n_frames, 2)
            self.assertEqual(preview.info["duration"], 70)
        with Image.open(output / "native-contact.png") as strip:
            self.assertEqual(strip.size, (44, 22))

    def test_cli_requires_existing_app_arguments_and_rejects_removed_modes(self):
        base = ["--title", "Gallery", "--targets", str(self.path), "--output", str(self.directory)]
        for args in ([], base + ["--samples", "x"], base + ["--seconds", "1"],
                     base + ["--position", "1", "2"]):
            with self.subTest(args=args):
                result = subprocess.run([sys.executable, str(Path(probe.__file__)), *args],
                                        capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn("required" if not args else "unrecognized arguments", result.stderr)


if __name__ == "__main__":
    unittest.main()
