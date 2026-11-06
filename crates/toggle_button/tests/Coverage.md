# Toggle Button Integration Test Coverage

## States and stimuli

Checked on/off is the sole state authority. Pointer press/release/cancel/click, focused Space/Enter, repeated/released/unrelated keys, local and ancestor Disabled, synchronous and queued setters, native Checked mutations, four themes and state color overrides exercise the public contracts.

## Guards and transitions

Disabled rejects user input and retains selection; removing ancestor Disabled restores interaction. Keyboard requires focus and a non-repeated activation key press. Press alone and cancellation do not toggle. Programmatic setting is permitted while disabled. Invalid or despawned targets return errors; queued requests commit at execution.

## Invariants and couplings

Dedicated notifications observe the already committed Checked state, and equal values are silent. Native Checked writes remain native and do not synthesize business events. Button role and toggled accessibility state track selection. Selected appearance persists after release; Disabled takes precedence over Pressed and Hovered. Palette fallback and clearing overrides follow the current theme, including managed child text foreground.

## Coverage map

`toggle.rs` covers these state, notification, input, theme, accessibility, error and content propagation contracts. BRP supplements tests with actual picking and rendered appearance; native OS mouse and screen-reader announcements remain outside this suite.

`headless.rs` exercises the public Scene/HeadlessPlugin without the default marker, children or foreground: initialization and same-value silence, pointer/keyboard input, commit-before-Changed, native Checked writes without notifications, local/ancestor Disabled and programmatic permissions, accessibility, and deferred validation after behavior identity changes. Installing the default Style alongside an existing Headless instance neither recolors it nor registers duplicate input commit observers.

`diagnostics.rs` covers silent unchecked/equal requests, typed invalid targets and operations, managed color ownership, Core color source/origin and state/field context, real queued state/color errors with Error severity and source identity, and Scene failure identity with root cleanup. Real style-system failures batch multiple damaged roots, retain each source ID while damage persists, and restore appearance with one recovery record per root after repair without requiring a selection change. Color module unit tests cover missing private ColorState without treating absent Checked as an error.
