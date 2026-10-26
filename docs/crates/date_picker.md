# DatePicker

`bevy_widgetry_date_picker` 提供 BSN `WidgetryDatePicker`、`WidgetryDatePickerProps` 和 `WidgetryDatePickerPlugin`，Facade 通过独立 `date_picker` Feature 与同名模块导出。

选中日期 `Option<Date>` 与显示年月 `DatePickerMonth { year, month }` 是两项独立私有状态。Props 仅初始化一次；显示年月优先显式值、初始选中日期、本地今天。本地日期不可用时记录 Warning，回退到 `min_date` 或当前 UTC 日期的月份，不自动选中也不伪造 Today。

通过 `selected_date` / `displayed_month` 查询，通过 `set_selected_date_in_world`、`set_displayed_month_in_world`、`shift_month_in_world` 同步修改，或使用无 `_in_world` 后缀的 Commands 入口。同步 Setter 返回 `Result<bool, BevyError>`；排队操作执行时校验。不同日期选择自动定位所属月份；同值选择不跳月；清空使用 `None` 且保留显示月份。浏览不改变选择。

`WidgetryDatePickerSelectionChanged` 和 `WidgetryDatePickerDisplayedMonthChanged` 仅在真实变化且状态已提交后触发，初始化与同值操作静默。程序化 API 不受 Disabled 限制。

`days(world, root, today)` 计算固定连续 42 个真实日期，包含邻月、选中、闭区间可选性和可选 Today 标识。`week_start` 默认 Monday，可使用任意 Weekday。边界与星期起点构造后固定，范围只限制选择。无法容纳完整 42 格的 `time::Date` 极端年月返回错误；无效配置、越界选择、无效根及缺失状态均明确报错，失败不改业务状态。

日期视图原位显示星期标题和固定 6×7 网格，顶部箭头按月浏览，底部「清空」在有值时可用。邻月有效日期可选择，越界日期无 Hover/Pressed 反馈。根或祖先有效 Disabled 阻止鼠标日期、翻月和清空操作。

四套 Theme 的 `date_picker` 配色提供普通、邻月、禁用文字及中性选中/Hover/Pressed 背景。`colors` Props 初始化 `WidgetryDatePickerColorOverrides`，其 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 支持合法局部覆盖与恢复主题。颜色变化不修改日期状态。

点击日期视图标题进入 3×4 月份视图，再点击年份标题进入每页连续 12 年的 3×4 年份视图。月份箭头按年、年份箭头按 12 年翻动候选；选择年份返回月份视图，确认月份才通过显示月份 API 提交并返回日期视图。候选浏览不改变业务状态、不发变化通知；高亮只表示已提交的显示年月，与选中日期独立。

三个视图共享 304×352 的默认外框，日期 42 格常驻，切换无弹窗、无动画。日期范围不限制候选浏览；无法表示完整月历的极端年月及超出 `time::Date` 的候选禁用。Disabled 阻止标题、候选及箭头操作，合法程序化更新仍可执行。

App 级 `WidgetryDatePickerLanguage` 呈现配置支持 `English`（默认）与 `SimplifiedChinese`，通过 Resource 原位更新星期、清空、月份及年份文案。语言变更保留选中日期、显示月份和未提交的候选视图，不重建实体、不发送业务变化事件。Gallery 将其与全局 Language 设置同步。

Plugin 自动安装内建 Asset 与 Core Icon 能力；创建 Scene 前应先安装 Plugin。日期格使用随方案交付的固定朱红色透明 SVG，文本外侧显示 Today 笔圈，可与选中底色叠加。笔圈不接受控件颜色覆盖，禁用、不可选和四套 Theme 均保留原色；月份及年份候选不显示笔圈。

Today 图标属于 DatePicker 托管部件，通用 Icon 颜色设置与清除 API 拒绝修改其固定 tint。

Plugin 共享可信本地日期缓存，使用系统单调时钟约每 60 秒检查一次；应用恢复更新后会立即检查已经到期的缓存。本地日期失败时记录一次 Warning 并移除 Today，持续失败不重复记录，成功恢复时记录 Info。Today 的变更、失效和恢复均不修改选择或显示年月、不发业务事件。

为使可休眠宿主实际执行检查，Plugin 在 Last 阶段将 Winit 两种 Reactive 模式的等待上限设为距下一次采样截止时间的一半，按秒向上取整、最长 30 秒，为 Windows 下 Bevy 0.20 的请求重绘及后续 App 更新各预留一次等待预算。保留宿主原有的更短等待、Continuous 及事件响应选项；临时缩短的等待在下一次采样后按原策略恢复，宿主更新等待策略时重新合并。没有 Winit 的 Headless App 仍在宿主驱动更新时检查日期。

宿主改变一种模式时保留另一模式的原始等待策略；仅修改事件响应选项也保留原等待时长。若检测到写入而两种模式的值均未改变，由于 Bevy 只提供 Resource 级变更检测，将其视为重新声明当前两种配置。
