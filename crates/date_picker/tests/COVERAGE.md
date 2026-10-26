# DatePicker coverage

`language.rs` switches English/Simplified Chinese in date, month and year views, retaining selection, committed month, temporary candidates and entity identity. It verifies weekday rotation, localized clear/month/year labels and absence of language-induced business events.

`headless.rs` covers independent selection/displayed-month states, initialization priority, continuous 42-day projection across leap/month/year boundaries and weekday starts, inclusive bounds, same-value and clear transitions, committed notifications, delayed Commands, invalid requests and Disabled programmatic updates.

Model unit tests inject local-date failure and UTC fallback to cover warning diagnostics without depending on the host clock. The projection accepts a separate optional Today input and never invents Today from fallback dates.

`view.rs` exercises real PointerClick routing to calendar cells, arrows and clear; adjacent/out-of-range dates, root/ancestor Disabled guards, and programmatic updates. It also verifies local color overrides survive four Theme changes, clearing restores Theme values, and invalid colors preserve the previous configuration.

PointerPress / PointerRelease exercise native Button pressed-state writers and their visual projection on dates, arrows and clear, including the Disabled press guard.

Clear-label color follows normal foreground while a selection is available, and disabled foreground after clearing or inheriting Disabled. The regression covers four themes and independent foreground/adjacent/disabled overrides, retaining adjacent-day styling.

`navigation.rs` covers the three inline views, temporary candidate year/page navigation without business changes, committed-month/year highlighting independent of selection, same-month confirmation, browsing beyond selection bounds, Disabled guards and stable outer dimensions.

Today unit tests inject monotonic deadlines and local-date reads to cover the one-minute cadence, midnight/resume changes, failure/recovery logging, removal of untrusted Today, shared projection across selected/bounded/disabled calendars without business notifications, and fixed icon tint under four themes. Reactive Winit wait limits reserve a timeout budget for both redraw and app update while preserving shorter waits, continuous updates and event flags; actual sleeping-loop wakeup is validated separately with BRP.

A headless UI-layout regression waits for the actual SVG raster, then checks the transformed image bounds and center against its day cell at 1x/2x scale; this protects against intrinsic-size image overflow.

The internal Today icon rejects public Icon color set/clear operations immediately after scene construction and flush, reports Error diagnostics, and retains its fixed normal/disabled white tint across subsequent view updates.

Host Winit policy regressions cover changing either focused or unfocused mode independently, editing event flags without losing original waits, and explicitly reasserting same-value settings after a temporary limit.
