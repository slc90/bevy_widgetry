# bevy_widgetry 测试覆盖模型

本模型记录当前自动化测试保护的主要行为契约与测试职责，不逐项列举测试用例。

## States
- Facade 与各可选控件 Plugin 的装配状态；Scene 构造、运行时选择、焦点、Disabled 和共享 Pointer 状态。
- Dialog／Window 的结果及生命周期、可选 Feature 的启用状态。

## Stimuli
- 从 Facade 导出 API 构造控件、安装 Plugin，调用公开命令及真实 Pointer／Keyboard 输入。
- Theme 更新、Scene 失败、焦点转移、Dialog 结果和资源关闭。

## Guards
- Scene、Plugin、Source 和 Parent 的前置条件有效；Disabled、Focus 及隐藏 Popup 的输入边界保持正确。
- 各测试按照所需 Feature 编译，不使某一可选控件成为其他 Feature 的隐含依赖。

## Transitions
- 组合控件构造 → 状态变更与事件派发 → 聚合资源清理；错误 Scene 经宿主传播并清理本次预约根。

## Invariants
- Facade 的导出不仅可见，而且能够实际构造并驱动对应控件；各模块保持原本的业务状态身份。
- Widgetry 组合装配保留宿主已有 Handler 与共享输入能力，不自动启用跨控件 Tab Navigation。
- 多控件组合的 Pointer Focus、键盘激活及通知归属不重复或串扰；错误转换保留原始诊断身份与领域原因。
- 可选独立窗口、资源和宏的公开入口在所需 Feature 下可正常使用。

## Couplings
- 可选 Feature × Facade 导出；多 Plugin 装配 × Focus／Pointer；跨控件场景 × 状态隔离与诊断传播。

## Coverage Map

### Integration Test

| 测试文件 | 主要测试职责 |
|---|---|
| `color_composition.rs` | 嵌套颜色作用域、自定义 Theme 和 Disabled 下的内容投影。 |
| `cursor.rs` | 聚合场景的 Cursor 与 Disabled／ReadOnly 声明。 |
| `diagnostics.rs` | 公共诊断导出、受管错误及宿主 Handler 协作。 |
| `focus.rs` | 跨控件焦点转移、可见状态和输入归属。 |
| `number_input.rs` | Facade 数值输入构造与公共行为。 |
| `plugin_focus.rs` | 共享输入插件与 Tab Navigation 的组合安装约束。 |
| `pointer.rs` | 跨控件真实 Pointer 路由及聚焦行为。 |
| `power_inhibit.rs` | 公开电源抑制能力的组合入口。 |
| `progress_bar.rs` | Facade 进度控件状态与构造入口。 |
| `public_api.rs` | Facade 公开 API、插件矩阵及 Theme／错误边界。 |
| `splash.rs` | 可选 Splash 的独立窗口创建与导出。 |
| `splitter.rs` | Facade Splitter 构造与公共操作。 |
| `tabs.rs` | Facade Tabs 的选择和交互契约。 |
| `taskbar_icon.rs` | 任务栏图标宏的资源嵌入与错误路径。 |
| `ui_scale.rs` | 全局 UI 缩放的聚合公开入口。 |
