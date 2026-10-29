from pathlib import Path
import runpy


if __name__ == "__main__":
    runpy.run_path(
        str(Path(__file__).resolve().parents[3] / "skills/animated-cursor-authoring/scripts/native_windows.py"),
        run_name="__main__",
    )
