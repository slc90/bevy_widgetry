# app_icon_build

## 功能概述

在应用构建期间，将调用方提供的 ICO 嵌入 Windows 可执行文件。该能力用于 EXE 本身的图标，不依赖运行时 Bevy App，也不需要随程序额外分发 ICO。

## 当前设计思路

这是独立的 Build Dependency，而不是 Widgetry 的运行时 Plugin。应用在自己的 `build.rs` 中调用 `set_exe_icon`，图标归应用所有，不由库分配或挑选。

## 主要功能

- `set_exe_icon(path)` 接收应用提供的 ICO 文件路径，将其交给 Windows 资源构建流程。
- 相对路径以调用方 `CARGO_MANIFEST_DIR` 为基准，不随当前工作目录变化。
- 为图标文件设置构建重运行依赖，内容变化后可重新嵌入。

## 行为与状态契约

构建步骤完成后，图标资源成为 EXE 的一部分。该函数只处理构建阶段的 ICO，不会为运行中的 Native Window、Taskbar 或 Widgetry UI 设置图标。窗口及任务栏图标由 `window` crate 的独立能力处理。

## 边界情况与错误处理

路径或资源处理失败通过 `io::Result` 返回，由应用构建脚本负责决定是否中断构建。

## 已知限制

- 不将 PNG、SVG 等图片自动转换为 ICO。
- 不提供运行时图标切换。
- 不是 `bevy_widgetry` Facade 的运行时 Feature；应用需要时单独引入 Build Dependency。
