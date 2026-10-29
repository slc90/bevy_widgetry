import json
from pathlib import Path
from types import SimpleNamespace
import sys
import tempfile
import unittest
from unittest.mock import patch

if sys.platform == "win32":
    import native_windows


class NativeSetupReached(Exception):
    pass


@unittest.skipUnless(sys.platform == "win32", "Windows native probe")
class NativeManifestTests(unittest.TestCase):
    def test_utf8_manifest_survives_a_non_utf8_windows_default(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            data = {"animations": [{"name": "冠", "atlas": "冠.png"}]}
            (directory / "manifest.json").write_text(
                json.dumps(data, ensure_ascii=False), encoding="utf-8"
            )
            original_read = Path.read_text

            def legacy_default(path, *args, **kwargs):
                if kwargs.get("encoding") is None:
                    kwargs["encoding"] = "cp936"
                return original_read(path, *args, **kwargs)

            # Stop before any window is created, after the real sample entry point decodes JSON.
            with patch.object(Path, "read_text", autospec=True, side_effect=legacy_default), patch.object(
                native_windows.user, "RegisterClassW", side_effect=NativeSetupReached
            ):
                with self.assertRaises(NativeSetupReached):
                    native_windows.sample(SimpleNamespace(samples=directory))


if __name__ == "__main__":
    unittest.main()
