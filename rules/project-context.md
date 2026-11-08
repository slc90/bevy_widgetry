# 项目背景与目标环境

## 目标平台

bevy_widgetry 当前仅支持以下环境：

- 操作系统：Windows 64 位。
- Rust Target：`x86_64-pc-windows-msvc`。
- 图形后端：DX12（wgpu backend）。
- 命令执行环境：Windows 下统一使用 PowerShell 7（`pwsh`），不使用 Windows PowerShell 5（`powershell.exe`）。

工程设计、实现和验证均以此环境为前提。

项目不以跨平台兼容为当前目标。除非任务明确要求改变平台支持范围，否则：

- 不为 Linux、macOS、Web 或 32 位系统增加兼容实现、fallback、stub 或条件分支。
- 不为假设中的其他平台、Target 或图形后端增加抽象层、依赖、Cargo Feature 或额外配置。
- 不为了保持非 Windows 平台可编译而增加当前需求不需要的代码。
- 可以直接使用 Windows 平台能力，不必为唯一支持的平台额外包裹 `#[cfg(windows)]`。

这些限制不意味着禁止通用代码或条件编译。没有额外维护成本的跨平台实现可以保留；当前需求确实需要的条件编译也可以使用。

**不为假设中的未来需求增加复杂度。** 如果以后需要支持其他平台、Target 或图形后端，应根据届时的实际需求重新设计，而不是提前预留兼容结构。

## 编译期平台检查

每个 Workspace crate 的入口必须包含：

```rust
#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");
```

Library 在 `lib.rs` 中检查，Application 在 `main.rs` 中检查，不在内部 Module 重复添加。

此检查用于明确拒绝不支持的 Target，不要求提供替代实现。

## UI 构造与组合

项目统一使用 **Bevy BSN** 声明、构造和组合 UI，包括 Widgetry 库及其 Gallery 等消费者。

BSN 是项目统一的 UI 组合方式，不应额外建立一套替代 BSN 的 UI 构造 API。

具体的 SceneComponent、Scene function、Props、Runtime ECS 操作及外部 Children 组合规则，遵循 `rules/ui-composition.md`。

## 输入与交互原则

Widgetry 采用以 Pointer 为主的用户交互模型，不提供跨独立控件的通用键盘焦点导航。

Pointer 输入负责用户选择、激活控件以及获取输入焦点。

键盘输入用于操作当前获得焦点的控件，包括文本编辑、数值调整、快捷操作及控件内部具有明确语义的交互。

程序化 API 可以根据业务需要主动设置或清除输入焦点。

BRP 自动化复用现有输入与 ECS 接口，不要求控件提供键盘导航路径。

Widgetry 不实现或自动安装 Tab / Shift+Tab 焦点遍历及跨控件方向键导航，不要求控件提供 TabIndex、TabGroup 或方向导航关系。

保留 Bevy 官方的输入焦点管理和焦点事件分发能力，不建立第二套焦点管理系统。

不为通用键盘导航、焦点遍历顺序或仅通过键盘操作完整界面的需求增加实现复杂度。

## Accessibility 约束

项目不以操作系统无障碍辅助技术支持为目标。Agent 对 UI 的自动化操作通过 BRP / MCP 实现，不依赖 Accessibility。

Widgetry 库及 Gallery 禁止主动新增或扩展 Accessibility / AccessKit 功能，包括：

- 为控件额外添加无障碍节点、角色、标签、状态或操作语义。
- 实现无障碍状态同步、事件处理及专用适配逻辑。
- 为无障碍功能引入额外的 Plugin、直接依赖、公共 API 或抽象层。

**Bevy 底层必要依赖除外。**

当使用 Bevy 官方 Component、Widget 或内部机制时，如果其正常构造、编译或运行确实需要 Accessibility 相关组件、类型或依赖，允许保留满足该要求的最小实现。

这类例外仅用于保持 Bevy 原生功能正常工作，不得借此扩展额外的无障碍功能。Bevy 自动提供的 Required Component 和传递依赖不需要主动移除。

本规则不限制正常的 Pointer、Keyboard、Focus、Picking、Widget 状态管理及 BRP 自动化能力。上述功能应通过各自的原生机制实现，不以 Accessibility 作为替代。

## 项目定位与兼容性原则

本项目主要供作者个人学习和使用，不以对外提供稳定的公共 API 为目标。

- 所有 crate 都不需要发布到 crates.io。
- 不要求保持 API 的向后兼容性。
- 不需要为历史版本提供兼容层、弃用过渡期或迁移方案。
- 允许根据设计需要直接进行破坏性重构，以设计合理性、实现简洁性和可维护性为优先。
- 只需保证代码在当前明确支持的 Bevy 版本下正常工作，不承担对旧版 Bevy 的兼容义务。
