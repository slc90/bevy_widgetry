# 补齐 Slider 拖动操作的语义结果日志

## 目标与设计思路

Slider 页面已经能统计用户拖动的开始、完成、取消及实际数值，但缺少 INFO，因而单靠日志无法了解用户最终确认了什么。本增量只在 Gallery 的**离散操作结尾**记录一次真实业务结果，不修改 Widgetry Slider 的拖动行为，也不把连续 ValueChange 变成日志流。

## 实际事件边界

`gallery/src/pages/slider.rs` 通过 `SliderDragStarted` 和 `SliderDragEnded` 更新 `SliderDemoState.started`、`completed`、`cancelled` 等计数；`ValueChange<f32>` 负责实时数值和 UI 更新。拖动过程中 ValueChange 可能频繁发生，因此不适合每次都记 INFO 或 DEBUG。`SliderDragEnded` 自带 `SliderDragOutcome::Completed | Cancelled` 和最终 `value`，可以直接说明操作结果。

在负责 `SliderDragEnded` 的 Gallery Observer 中记录一次 INFO，包含控件 Entity 或现有稳定 Name、真实 `outcome`、最终 `value`。完成和取消必须使用不同结果，不得将取消写成“正式提交”；即使值与拖动开始时相同，只要用户确实完成了可识别的拖动操作，也记录“完成但未变化”的事实。已有 `SliderDemoState` 继续作为 UI 计数与状态的真实来源，不用日志来代替它。

对于通过键盘产生的非拖动值改变，应先确认 Slider 的实际公共契约是否提供离散的用户步进／确认事件。如果只能观察到连续 `ValueChange`，不能凭空断言每次通知就是一次完整业务提交；可在有清晰应用语义边界的操作上补 INFO，但不为此新增低层 Pointer／Keyboard 全局监听，不修改控件行为与事件发射约定。

新增日志正文统一中文，动态上下文使用英文结构化字段；只记录必要的对象身份、状态和结果摘要，不打印完整用户输入、敏感内容或大体量业务数据，也不手动重复 Tracing 已提供的 `file`、`line`、`module_path`、`crate_name` 等元数据。

## 验证与边界

通过已有 Slider 测试状态和必要的 Gallery GUI／BRP 操作，验证一次完成、一次取消、未发生数值变化但正常结束、长距离连续拖动的日志数量；拖动中多次 ValueChange 不得产出同等数量的 INFO。日志正文中文、结构化字段英文且只包含必要值，不增加坐标轨迹或完整输入事件录制。默认 Hover、Focus、Pointer Move 保持静默，真实错误仍遵循 `rules/code.md`。

同步 `docs/gallery.md` 中 Slider 交互章节对实际可见日志的说明；本次不增加通用 Slider 内部日志。
