# DatePicker coverage

`headless.rs` covers independent selection/displayed-month states, initialization priority, continuous 42-day projection across leap/month/year boundaries and weekday starts, inclusive bounds, same-value and clear transitions, committed notifications, delayed Commands, invalid requests and Disabled programmatic updates.

Model unit tests inject local-date failure and UTC fallback to cover warning diagnostics without depending on the host clock. The projection accepts a separate optional Today input and never invents Today from fallback dates.
