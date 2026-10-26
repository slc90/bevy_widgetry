# DatePicker

`bevy_widgetry_date_picker` 提供 BSN `WidgetryDatePicker`、`WidgetryDatePickerProps` 和 `WidgetryDatePickerPlugin`，Facade 通过独立 `date_picker` Feature 与同名模块导出。

选中日期 `Option<Date>` 与显示年月 `DatePickerMonth { year, month }` 是两项独立私有状态。Props 仅初始化一次；显示年月优先显式值、初始选中日期、本地今天。本地日期不可用时记录 Warning，回退到 `min_date` 或当前 UTC 日期的月份，不自动选中也不伪造 Today。

通过 `selected_date` / `displayed_month` 查询，通过 `set_selected_date_in_world`、`set_displayed_month_in_world`、`shift_month_in_world` 同步修改，或使用无 `_in_world` 后缀的 Commands 入口。同步 Setter 返回 `Result<bool, BevyError>`；排队操作执行时校验。不同日期选择自动定位所属月份；同值选择不跳月；清空使用 `None` 且保留显示月份。浏览不改变选择。

`WidgetryDatePickerSelectionChanged` 和 `WidgetryDatePickerDisplayedMonthChanged` 仅在真实变化且状态已提交后触发，初始化与同值操作静默。程序化 API 不受 Disabled 限制。

`days(world, root, today)` 计算固定连续 42 个真实日期，包含邻月、选中、闭区间可选性和可选 Today 标识。`week_start` 默认 Monday，可使用任意 Weekday。边界与星期起点构造后固定，范围只限制选择。无法容纳完整 42 格的 `time::Date` 极端年月返回错误；无效配置、越界选择、无效根及缺失状态均明确报错，失败不改业务状态。

当前提供 Headless 日期状态与投影；尚未提供日期 UI、快速年月定位及本地 Today 更新。
