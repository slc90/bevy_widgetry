# DatePicker coverage

`headless.rs` covers independent selection/displayed-month states, initialization priority, continuous 42-day projection across leap/month/year boundaries and weekday starts, inclusive bounds, same-value and clear transitions, committed notifications, delayed Commands, invalid requests and Disabled programmatic updates.

Model unit tests inject local-date failure and UTC fallback to cover warning diagnostics without depending on the host clock. The projection accepts a separate optional Today input and never invents Today from fallback dates.

`view.rs` exercises real PointerClick routing to calendar cells, arrows and clear; adjacent/out-of-range dates, root/ancestor Disabled guards, and programmatic updates. It also verifies local color overrides survive four Theme changes, clearing restores Theme values, and invalid colors preserve the previous configuration.

PointerPress / PointerRelease exercise native Button pressed-state writers and their visual projection on dates, arrows and clear, including the Disabled press guard.

`navigation.rs` covers the three inline views, temporary candidate year/page navigation without business changes, committed-month/year highlighting independent of selection, same-month confirmation, browsing beyond selection bounds, Disabled guards and stable outer dimensions.
