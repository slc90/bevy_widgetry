# bevy_widgetry

## 功能概述

为应用提供 Widgetry 的统一公共入口，集中导出基础 Widget、数据视图、窗口组合与共享 UI 能力。

## 当前设计思路

这是 Facade，不承载各 Widget 的行为实现。控件和 Model 位于独立 crate，Facade 通过模块对外提供调用入口；应用仍按各 Widget 要求安装 Plugin、注册 Typed Renderer 或提供 Model。

## 主要功能

- 基础控件：Button、CheckBox、RadioGroup、Segmented、Tabs、Slider、NumberInput、TextField、ComboBox、Tooltip。
- 数据与显示：ProgressBar、Splitter、ScrollArea、ListView、Tree、Table、Waveform。
- 窗口与 Dialog：Window、MessageBox、FileDialog。
- 通用能力：Theme、Text、Icon、Disabled 有效状态、Cursor、Style、Scene 错误处理扩展。
- 提供 `window::taskbar_icon!`，在编译期嵌入调用方提供的 PNG，并安装主窗口/任务栏图标能力。

## 行为与状态契约

Widget 使用 BSN Scene 与一次性 Props 构造。运行时 State、通知与输入契约由对应 Widget 定义，不由 Facade 建立第二份全局状态。`scene` 模块提供可报告并清理失败预约实体的 Scene 操作入口，错误交由宿主 Handler。

## 按需编译

默认 Feature 启用全部 22 个控件模块，原有默认用法保持可用。每个控件的 Feature、公开模块和直接可选依赖一一对应：`button`、`hold_button`、`toggle_button`、`check_box`、`radio_group`、`segmented`、`tabs`、`slider`、`number_input`、`progress_bar`、`splitter`、`text_field`、`combo_box`、`tooltip`、`scroll_area`、`list_view`、`tree`、`table`、`waveform`、`window`、`message_box`、`file_dialog`。

精简应用可关闭默认集合，再显式选择直接使用的模块：

```toml
bevy_widgetry = { path = "../bevy_widgetry/crates/bevy_widgetry", default-features = false, features = ["button", "text_field"] }
```

Feature 控制 Facade 的直接依赖和公共入口，保留控件自身的真实传递依赖。例如 `combo_box` 内部仍编译 ListView 等组合控件，但不会自动开放 Facade 的 `list_view` 模块；应用直接使用该模块时须显式启用 `list_view`。

`scene`、`text`、`icon`、`disabled`、`style`、`theme`、`cursor` 是始终可用的基础入口。Core 本身依赖 Theme、Cursor 和内建 Asset，因此无默认 Feature 仍会编译这些基础设施。内建 Asset、内部日志和测试设施不作为控件 Feature 导出。`app_icon_build` 仍是独立 Build Dependency。

集成测试按实际使用的控件组合启用；`pointer_lifecycle` Benchmark 要求 `button`、`list_view`、`scroll_area`、`table`。无默认集合的 `--all-targets` 不会无条件导入关闭的模块。

独立的 `app_logging` Feature 默认不启用，通过 `app_logging` 模块重导出已有 `AppLogging`、`file_layer`、`terminal_layer`、`WorkerGuard`。它可以与任意控件 Feature 组合，也可在无默认控件时单独启用。仅启用 Feature 不会安装 Subscriber；宿主仍须先准备日志、`install`，再装配官方 LogPlugin，并持有 Guard 直到退出。

## 边界情况与错误处理

- Widget/Scene 初始化失败必须依据各自契约向宿主传递，而不是被 Facade 吞掉。
- 某些功能需要显式 Plugin、数据类型注册或 Render/Asset 环境；仅 import Facade 不保证自动完成业务装配。
- `taskbar_icon!` 的图片输入由调用方提供，无效素材在安装前报告错误。

## 已知限制

- 关闭默认集合能减少非必要控件依赖，但不移除基础设施或所选控件的必要传递依赖。
- EXE ICO 嵌入属于独立 `app_icon_build` 构建期能力，不通过 Facade 提供。
- 具体控件边界见各 `docs/crates/` 文档。


`hold_button` Feature 默认启用，经 `bevy_widgetry::hold_button` 导出长按控件、Plugin、Props 和颜色覆写类型。
