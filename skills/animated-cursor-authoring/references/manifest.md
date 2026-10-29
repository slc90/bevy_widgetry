# Atlas manifest and tools

Both helpers require Python with Pillow and NumPy. A tested invocation is:

```pwsh
uv run --python 3.13 --with pillow --with numpy python <skill>/scripts/cursor_tool.py manifest.json --output preview
```

The manifest is UTF-8 JSON and is theme-independent. Semantic names and atlas filenames may contain Unicode. Atlas paths resolve relative to the manifest. Each atlas is an exact RGBA8 grid with a transparent border around each frame and zero RGB wherever alpha is zero.

```json
{
  "animations": [
    {
      "name": "Inspect",
      "atlas": "inspect.png",
      "frame_size": [48, 48],
      "columns": 4,
      "frames": 12,
      "frame_duration_ms": 80,
      "hotspot": [10, 10],
      "anchor_rect": [9, 9, 12, 12],
      "motion": "A scan travels beside a stationary inspection point."
    }
  ]
}
```

Use `durations_ms` instead of `frame_duration_ms` for variable frame durations; provide one positive millisecond duration per frame. Sizes, columns, frame counts, timings, and hotspots may vary between animations. `anchor_rect` is optional and uses exclusive right/bottom coordinates. It protects a deliberately fixed pixel patch, not the entire functional silhouette. Missing anchor checks are reported as unchecked. All previews loop continuously. The helper validates total duration even if APNG merges identical held frames.

Inspect `index.html` in a browser, using both backgrounds. Check `qc.json`, especially `seam_needs_attention`; a high ratio can indicate a reset jump or an intentional transition. Ratios are heuristic. Native captures and art judgments remain separate.

## Windows sample playback

With `manifest.json` and its atlases in a directory:

```pwsh
uv run --python 3.13 --with pillow --with numpy python <skill>/scripts/native_windows.py --samples samples --output native-samples
```

This opens a temporary native window, moves the OS mouse into it, loads CUR frames, plays two loops per semantic with its own timing, and records actual native bitmaps and hotspot/size observations. It restores the mouse position and destroys the temporary cursor handles/window afterward. CUR sample frames support dimensions up to 256px. Avoid concurrent physical mouse input during this short observation.

## Observe a real application's cursor

Prepare a JSON array of target entries. Each entry has the manifest's animation fields plus `position: [x,y]` in physical client pixels. Atlas paths resolve relative to this target JSON. Navigate the application normally before recording the currently visible targets; avoid inventing ECS state to imitate interaction.

```pwsh
uv run --python 3.13 --with pillow --with numpy python <skill>/scripts/native_windows.py --title "Target application" --targets targets.json --foreground --output native-app
```

`--foreground` temporarily raises the existing application so a covered window cannot produce another application's cursor. It does not navigate, click, scroll, or mutate the application's state. Capture without this option when the window is already unobstructed. The title must match the live native window title, including localization.

Each semantic gets an output directory with `native.json`, `native.apng`, and a motion strip. Capture validates native dimensions, fixed hotspots, animation changes, and source frame membership. Bitmap hashing accepts straight BGRA and the common premultiplied rounding variants; inspect the consumer's actual native representation if another encoding is used. `native.json` keeps real sample times; the recorded APNG follows those sample intervals. This establishes native output identity, not click accuracy, semantic priority, focus lifecycle, or frame-perfect display latency.
