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

## 项目定位与兼容性原则

本项目主要供作者个人学习和使用，不以对外提供稳定的公共 API 为目标。

- 所有 crate 都不需要发布到 crates.io。
- 不要求保持 API 的向后兼容性。
- 不需要为历史版本提供兼容层、弃用过渡期或迁移方案。
- 允许根据设计需要直接进行破坏性重构，以设计合理性、实现简洁性和可维护性为优先。
- 只需保证代码在当前明确支持的 Bevy 版本下正常工作，不承担对旧版 Bevy 的兼容义务。
