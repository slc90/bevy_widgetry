# 模块核心设计

每篇从所属能力的核心问题出发，逐步解释状态权威、组合关系和生命周期。跨模块公共协议在机制目录中展开。

## 入口与内容基础

| 文档 | 核心问题 |
| --- | --- |
| [统一入口](bevy_widgetry.md) | 统一公开入口与独立实现的关系 |
| [Core](core.md) | 共享 UI 协调而非全局业务模型 |
| [内建资源](asset.md) | 语义身份、注册与解析的分工 |
| [颜色主题](theme.md) | 完整主题提交与控件自主投影 |
| [光标](cursor.md) | 语义、临时控制权与最终呈现 |

## 按钮与选择

| 文档 | 核心问题 |
| --- | --- |
| [Button](button.md) | 统一激活、独立行为与默认组合 |
| [HoldButton](hold_button.md) | 尝试会话、计时和成功锁存 |
| [ToggleButton](toggle_button.md) | Checked 权威与视觉投影 |
| [CheckBox](check_box.md) | 不同状态模型与共享外观 |
| [RadioGroup](radio_group.md) | 成员结构与唯一选择 |
| [Segmented](segmented.md) | 选择、结构维护与浮块呈现 |
| [DatePicker](date_picker.md) | 所选日期与显示月份独立 |

## 输入与布局

| 文档 | 核心问题 |
| --- | --- |
| [TextField](text_field.md) | 逻辑根、实际 Editor 与内容状态 |
| [NumberInput](number_input.md) | 正式数值与文本草稿的提交边界 |
| [Slider](slider.md) | 数值能力、输入会话与视觉分离 |
| [ProgressBar](progress_bar.md) | 应用进度与展示动画的边界 |
| [Splitter](splitter.md) | 目标比例与受约束几何 |
| [Tabs](tabs.md) | 固定配对与常驻页面生命周期 |
| [Collapsible](collapsible.md) | 唯一展开状态与常驻内容 |
| [ScrollArea](scroll_area.md) | 滚动状态、视口和滚动条关系 |
| [Tooltip](tooltip.md) | 全局显示会话和局部锚点 |

## 数据视图

| 文档 | 核心问题 |
| --- | --- |
| [ListView](list_view.md) | 稳定数据身份与虚拟化 Row |
| [ComboBox](combo_box.md) | 共享数据、唯一选择和开关状态 |
| [Tree](tree.md) | 业务层级和逐 View 可见投影 |
| [Table](table.md) | 稳定 Cell 身份与视图级交互 |
| [Canvas](canvas.md) | Document、View、草稿和渲染分离 |
| [Waveform](waveform.md) | 时间、请求身份与完整显示快照 |
| [磁盘波形源](waveform_file.md) | 文件索引、Reader 隔离与真实退出 |

## 窗口与桌面能力

| 文档 | 核心问题 |
| --- | --- |
| [Window](window.md) | 原生绑定、拥有关系与输入作用域 |
| [MessageBox](message_box.md) | 单次决议和资源回收顺序 |
| [FileDialog](file_dialog.md) | Session、稳定身份与异步结果采用 |
| [窗口缩放](ui_scale.md) | 逻辑尺寸基准和真实窗口几何 |
| [Splash](splash.md) | 内容就绪与真实首帧的握手 |
| [Tray](tray.md) | 原生实例、回调路由与应用策略 |
| [电源抑制](power_inhibit.md) | 业务需求与原生请求所有权 |

## 任务、诊断与工程辅助

| 文档 | 核心问题 |
| --- | --- |
| [后台任务](job.md) | 执行、终态和订阅的独立生命周期 |
| [诊断](diagnostics.md) | 来源身份与传播语境分离 |
| [内部日志](internal_log.md) | 发出端与宿主接收端分工 |
| [应用日志](app_logging.md) | 异步写入、健康状态和关闭责任 |
| [EXE 图标](app_icon_build.md) | 构建资源与运行时对象分离 |
| [测试辅助](test_utils.md) | 真实系统与可控测试环境 |

[整体设计](../architecture.md) · [跨模块机制](../mechanisms/README.md) · [功能说明](../../features/README.md)
