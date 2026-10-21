# Toggle Button Integration Test Coverage

## States and stimuli

Checked on/off is the sole state authority. Pointer press/release/cancel/click, focused Space/Enter, repeated/released/unrelated keys, local and ancestor Disabled, synchronous and queued setters, native Checked mutations, four themes and state color overrides exercise the public contracts.

## Guards and transitions

Disabled rejects user input and retains selection; removing ancestor Disabled restores interaction. Keyboard requires focus and a non-repeated activation key press. Press alone and cancellation do not toggle. Programmatic setting is permitted while disabled. Invalid or despawned targets return errors; queued requests commit at execution.

## Invariants and couplings

Dedicated notifications observe the already committed Checked state, and equal values are silent. Native Checked writes remain native and do not synthesize business events. Button role and toggled accessibility state track selection. Selected appearance persists after release; Disabled takes precedence over Pressed and Hovered. Palette fallback and clearing overrides follow the current theme, including managed child text foreground.

## Coverage map

`toggle.rs` covers these state, notification, input, theme, accessibility, error and content propagation contracts. BRP supplements tests with actual picking and rendered appearance; native OS mouse and screen-reader announcements remain outside this suite.
