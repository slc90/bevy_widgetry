# HoldButton coverage

- `interaction.rs`: real UI layout and Picking for Mouse and Custom Pointer; transparent corner pass-through, content hit routing, short press rejection, immediate one-shot Activate at full charge, release/repress cycles, drag-out/re-entry lockout and ancestor Disabled/Visibility recovery. Existing nested content reparenting preserves Picking pass-through. Verifies the absence of native Button and default TabIndex.
- Internal Headless unit tests: linear charge/decay, clamping and zero Duration, partial resume, success latch, idle wake-up and delayed Virtual delta exclusion, release-at-threshold settlement and feedback, same-frame in-place Hidden / Display::None interruption priority; ECS keyboard focus, repetition, mixed sources and state interruption contracts without exporting progress.
- Internal Style unit tests: Theme / instance override / clear, success flash duration and idle redraw behavior.
- GPU shader appearance and animation timing are additionally checked in the Gallery via BRP, outside the headless suite.
