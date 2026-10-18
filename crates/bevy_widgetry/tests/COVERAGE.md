# bevy_widgetry Integration Test Coverage

本覆盖模型描述现有 Integration Test 重点保护的可观察行为及测试职责，不是逐项 Test Case 清单。

## States

- Facade Plugin 未装配 / 已装配；Scene 待构造 / 成功 / 失败。
- 控件 Selection、Focus、Disabled 与共享 Pointer 输入状态。
- Dialog 未决议 / 已决议 / 生命周期结束。

## Stimuli

- 从 Facade 构造各 Widget Scene、注册 Typed Renderer / Plugin。
- 程序 API、Mouse / Custom Pointer、Keyboard、Theme、Scene 失败和窗口结果动作。

## Guards

- Source、Parent、Plugin 及 Scene 配置有效性。
- Disabled、Focus 和隐藏 Popup 对输入的约束。
- 各集成测试仅在自身使用的控件 Feature 全部启用时编译；默认 Feature 保留全部现有行为覆盖。

## Transitions

- 公开 Scene 创建、Widget 状态改变、Input Focus 转移。
- Modal / Dialog 操作结束并清理自有资源。
- Scene 失败交给宿主 Handler，回收本次预约 Entity。

## Invariants

- Facade 模块可实际构造并消费 Widget，而不是仅在编译期可见。
- 相同 Source 的 ListView Selection 独立；同帧 Focus / Popup 输入归属正确。
- Pressed Cleanup 不清除非 Owner 的程序化 Pressed。
- 插件装配不覆盖宿主的 Font 策略。
- TextField 的根内容 API、内部编辑实体、Prefix/Suffix 公共类型及编辑区域 Cursor 仍能通过 Facade 正确组合。
- 单独 / 组合控件安装复用已有基础 UI Plugin；CheckBox、RadioGroup、Tooltip 的注册完成日志各只记录一次。

## Couplings

- 共享 UI / Pointer Plugin 与各基础控件的组合。
- TextField / ComboBox 的 Focus 抢占与同帧 Keyboard。
- Scene 失败、Window 和 Dialog 的真实资源归属。
- FileDialog 关闭先保留 native Window 跨过通知帧，再回收 Root / Window / Camera，与现有两阶段关闭契约一致。
- Foreground 在独立 Widget 重建作用域，Disabled 继续跨 Widget 继承；TextColor 同帧正确，原生 Text 保留显式颜色。
- 两种 Plugin 顺序下，Popup 关闭后的同帧 Keyboard 仅交给当前 TextField，隐藏列表的状态和通知保持静默。
- Picking 后注入非 Owner 的高层 Cancel / Release / DragEnd，分别保护 Pointer / Target / Button Ownership；程序化 Pressed 保留。
- ListView 物化 Row 的显式 Default Cursor 截断祖先 Pointer，Row 内 Button 仍可声明自身 Cursor；Custom 不控制 OS Cursor。
- 调用方 package 的 PNG 编译嵌入通过 Facade Macro 使用，无效内容在安装前以 Severity::Error 失败，不依赖运行期原文件或 AssetServer。

## Coverage Map

- `public_api.rs`：Facade 导出、各 Widget Scene / State、Button Loading API 与变化通知的公开入口、插件组合及 Dialog Lifecycle。
- `focus.rs`：跨 Widget Focus 与隐藏 Popup 的输入隔离。
- `pointer.rs`：真实 Pointer Pipeline 中 Button / CheckBox / Radio 等 Pressed Ownership。
- `cursor.rs`：基础 Widget Cursor 与继承 Disabled、ReadOnly 的耦合。
- `color_composition.rs`：嵌套 Widget Foreground Scope、颜色与 Disabled 继承。
- `taskbar_icon.rs`：Facade Macro 的调用方 PNG 嵌入和失败路径。

