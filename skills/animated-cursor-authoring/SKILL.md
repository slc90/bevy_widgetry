---
name: animated-cursor-authoring
description: Design, produce, integrate, and verify looping animated mouse cursor themes with stable functional anchors. Use for creating or revising cursor artwork and timing, native cursor playback, or a reusable cursor production pipeline.
---

# Animated Cursor Authoring

Treat the semantic inventory, canvas size, functional anchors, target runtime, and reference art as inputs. Do not inherit an example theme's palette, enum, timing, directory structure, or engine version.

## Establish the contract

Inspect the actual consumer before choosing a production method. Determine how it interprets RGBA pixels, atlas crops, hotspot coordinates, animation clocks, fallback, focus, dragging, capture, and native window operations. An accepted asset or input request does not prove a displayed OS cursor.

Create a compact per-semantic motion brief: what stays fixed, what moves, why that movement communicates the operation, and the intended loop duration. Directional families can share motion when their semantics justify it. A common whole-image pulse is usually a poor substitute for designing the tools and gestures.

Choose representative samples with different risks, such as a precise insertion point, directional action, ordinary pointer, and busy state. Test those at the requested output size and in the actual native environment before expanding the collection. Select vector drawing, layered raster rigs, image generation, frame drawing, interpolation, or a combination based on results; no particular technique is mandatory.

## Produce and inspect

Keep functional silhouettes and hotspots stationary. Animate secondary geometry, local limbs, illumination, or indicators around them. A metadata coordinate staying constant does not prove that the visible tip or insertion point stays constant.

Retain high-resolution or vector sources and reproducible export code. Downsample once. Leave deliberate transparent padding, clear hidden RGB at fully transparent pixels, and inspect edges against both light and dark surfaces. Protect precision anchors through the downsampling filter's footprint; antialiasing can sample nearby moving pixels even when the nominal anchor geometry is fixed.

Set frame counts and durations for each gesture. Avoid abrupt reset jumps: close the motion path, return smoothly, or make a travelling accent disappear before restarting. Holds and repeated frames can be intentional. Inspect continuous loops at native size, motion strips, and enlarged edges; do not equate a successful script or seam metric with art approval.

Use [the manifest and tooling reference](references/manifest.md) when exporting atlases. `scripts/cursor_tool.py` validates the actual atlas pixels, optional protected anchor patches, and timing, then produces an animated HTML contact sheet, APNGs, motion strips, and a report. Its seam ratio identifies candidates for inspection and does not decide artistic quality.

## Integrate and verify

Feed native playback and previews from the same semantic assets and timing metadata. Keep playback identity separate from frame index, preserve phase while a semantic stays active, and define restart behavior at semantic changes. Test variable rates, a stationary pointer, fallback and recovery, focus loss, native ownership, capture/drag, theme changes, caller-provided custom images, and window/page teardown as applicable to the host.

When assets load on demand, test a cold first hover with a stationary OS mouse before running a preload gallery. Keep necessary updates while the current asset is actually loading, or arrange a completion wake. A failed request or missing pixels in an already loaded asset must not keep that loading wake alive.

Measure the real simultaneous preview workload. Confirm every required preview advances, including offscreen previews when that is part of the contract. Check that inactive pages and unavailable animations stop requesting unnecessary work. Separate CPU update, rendering/presentation, and native input evidence.

On Windows, the optional `scripts/native_windows.py` can play sample atlases in a temporary native window or observe an existing application's OS cursors through `GetCursorInfo` and `GetIconInfo`. Existing-app capture also compares native bitmap hashes against the expected source frames. The sample mode proves native format/size/hotspot viability; it does not prove the consumer's semantic mapping or interactions. Existing-app positions use physical client pixels. The helper briefly nudges before positioning because assigning the current position may not restore a Mouse identity after a simulator releases it. Use the target platform's equivalent observation when Windows is not the target; the helpers do not require Windows for ordinary atlas validation.

Verify actual click, text insertion, dragging, and resize precision where applicable. A simulated picking pointer is separate from an OS mouse. Keep the host project's required GUI validation, and supplement it with native evidence. Record limitations explicitly when a native environment is unavailable.

Deliver source art, runtime assets, timing/hotspot metadata, reproducible commands, directly inspectable loops, and validation evidence. Distinguish technical checks, visual review, native format probes, and actual application acceptance. For a worked example, read [the Kamuri case study](references/kamuri-case-study.md) only when useful.
