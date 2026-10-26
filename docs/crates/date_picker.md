# DatePicker

`bevy_widgetry_date_picker` 提供 BSN `WidgetryDatePicker`、`WidgetryDatePickerProps` 和 `WidgetryDatePickerPlugin`，Facade 通过独立 `date_picker` Feature 与同名模块导出。

选中日期 `Option<Date>` 与显示年月 `DatePickerMonth { year, month }` 是两项独立私有状态。Props 仅初始化一次；显示年月优先显式值、初始选中日期、本地今天。本地日期不可用时记录 Warning，回退到 `min_date` 或当前 UTC 日期的月份，不自动选中也不伪造 Today。

通过 `selected_date` / `displayed_month` 查询，通过 `set_selected_date_in_world`、`set_displayed_month_in_world`、`shift_month_in_world` 同步修改，或使用无 `_in_world` 后缀的 Commands 入口。同步 Setter 返回 `Result<bool, BevyError>`；排队操作执行时校验。不同日期选择自动定位所属月份；同值选择不跳月；清空使用 `None` 且保留显示月份。浏览不改变选择。

`WidgetryDatePickerSelectionChanged` 和 `WidgetryDatePickerDisplayedMonthChanged` 仅在真实变化且状态已提交后触发，初始化与同值操作静默。程序化 API 不受 Disabled 限制。

`days(world, root, today)` 计算固定连续 42 个真实日期，包含邻月、选中、闭区间可选性和可选 Today 标识。`week_start` 默认 Monday，可使用任意 Weekday。边界与星期起点构造后固定，范围只限制选择。无法容纳完整 42 格的 `time::Date` 极端年月返回错误；无效配置、越界选择、无效根及缺失状态均明确报错，失败不改业务状态。

日期视图原位显示星期标题和固定 6×7 网格，顶部箭头按月浏览，底部「清空」在有值时可用。邻月有效日期可选择，越界日期无 Hover/Pressed 反馈。根或祖先有效 Disabled 阻止鼠标日期、翻月和清空操作。

四套 Theme 的 `date_picker` 配色提供普通、邻月、禁用文字及中性选中/Hover/Pressed 背景。`colors` Props 初始化 `WidgetryDatePickerColorOverrides`，其 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 支持合法局部覆盖与恢复主题。颜色变化不修改日期状态。

当前尚未提供快速年月定位及本地 Today 更新。
