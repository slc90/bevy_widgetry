# bevy_widgetry

## 功能概述

为应用提供 Widgetry 的统一公共入口，集中导出基础 Widget、数据视图、窗口组合与共享 UI 能力。

## 当前设计思路

这是 Facade，不承载各 Widget 的行为实现。控件和 Model 位于独立 crate，Facade 通过模块对外提供调用入口；应用仍按各 Widget 要求安装 Plugin、注册 Typed Renderer 或提供 Model。

## 主要功能

- 基础控件：Button、CheckBox、RadioGroup、TextField、ComboBox、Tooltip。
- 数据与显示：ScrollArea、ListView、Tree、Table、Waveform。
- 窗口与 Dialog：Window、MessageBox、FileDialog。
- 通用能力：Theme、Text、Icon、Disabled 有效状态、Cursor、Style、Scene 错误处理扩展。
- 提供 `window::taskbar_icon!`，在编译期嵌入调用方提供的 PNG，并安装主窗口/任务栏图标能力。

## 行为与状态契约

Widget 使用 BSN Scene 与一次性 Props 构造。运行时 State、通知与输入契约由对应 Widget 定义，不由 Facade 建立第二份全局状态。`scene` 模块提供可报告并清理失败预约实体的 Scene 操作入口，错误交由宿主 Handler。

## 边界情况与错误处理

- Widget/Scene 初始化失败必须依据各自契约向宿主传递，而不是被 Facade 吞掉。
- 某些功能需要显式 Plugin、数据类型注册或 Render/Asset 环境；仅 import Facade 不保证自动完成业务装配。
- `taskbar_icon!` 的图片输入由调用方提供，无效素材在安装前报告错误。

## 已知限制

- 当前各公开模块通过顶层 Facade 统一导出；按控件可选编译的 Feature 划分尚未实现。
- EXE ICO 嵌入属于独立 `app_icon_build` 构建期能力，不通过 Facade 提供。
- 具体控件边界见各 `docs/crates/` 文档。

