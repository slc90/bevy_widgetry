# Theme integration coverage

- `runtime_theme.rs` exercises an external complete palette through the public World and queued APIs, including delayed execution, consecutive changes, committed state visible to observers, and equal-value silence.
- Invalid nested colors and invalid waveform palettes must reject the whole palette without state mutation or notification; existing preset contrast and transition checks remain in the source unit tests.
- Missing resources and command error diagnostics are covered by `test_utils::theme`.
