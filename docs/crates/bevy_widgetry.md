# bevy_widgetry

## 功能概述

为应用提供 Widgetry 的统一公共入口，集中导出基础 Widget、数据视图、窗口组合与共享 UI 能力。

## 当前设计思路

这是 Facade，不承载各 Widget 的行为实现。控件和 Model 位于独立 crate，Facade 通过模块对外提供调用入口；应用仍按各 Widget 要求安装 Plugin、注册 Typed Renderer 或提供 Model。

## 主要功能

- 基础控件：Button、CheckBox、RadioGroup、Segmented、Tabs、Slider、NumberInput、TextField、ComboBox、Tooltip。
- 数据与显示：ProgressBar、Splitter、ScrollArea、ListView、Tree、Table、Waveform、Canvas。
- 窗口与 Dialog：Window、MessageBox、FileDialog。
- 通用能力：Theme、Text、Icon、Disabled 有效状态、Cursor、Style、Scene 错误处理扩展。
- `diagnostics` 始终导出统一领域失败及可选 Bevy ECS 桥接能力，Facade 显式启用下层 `bevy` Feature；转换保留 source/origin/多失败，并不安装宿主 Handler。详见 [Diagnostics](diagnostics.md)。
- 提供 `window::taskbar_icon!`，在编译期嵌入调用方提供的 PNG，并安装主窗口/任务栏图标能力。
- 默认关闭的 `tray` Feature 提供 `tray::WidgetryTrayPlugin`，接受嵌入 ICO 与静态 Tooltip；需显式添加插件，详见 [Tray](tray.md)。
- 默认关闭的 `power_inhibit` Feature 提供 `power_inhibit::{WidgetryPowerInhibitPlugin, PowerInhibit, PowerError, PowerFailure, PowerFailures, PowerOperation, PowerPhase, RequestKind}`，由宿主通过同步 API 控制 Windows 空闲息屏和休眠抑制，并在 ECS 边界显式转换领域失败集合，详见 [PowerInhibit](power_inhibit.md)。
- 默认关闭的 `job` Feature 经 `job` 模块重导出 JobManager、Plugin、类型化结果接收与关闭协调；执行实现位于独立基础设施 crate，详见 [Job](job.md)。

## 行为与状态契约

Widget 使用 BSN Scene 与一次性 Props 构造。运行时 State、通知与输入契约由对应 Widget 定义，不由 Facade 建立第二份全局状态。`scene` 模块提供可报告并清理失败预约实体的 Scene 操作入口，错误交由宿主 Handler。

`scene::{spawn_scene, apply_scene}` 同步返回 `SceneFailure`，由调用方在执行边界显式 `into_bevy_error()`；同步调用不自动调用 Handler。Facade 同时重导出原始 Scene 包装读取与源头适配入口，Deferred 入口仍负责真实错误转交和预约清理。

## 按需编译

`ui_scale` Feature 默认关闭，通过 `ui_scale` 模块提供普通窗口的全局应用比例及插件，详见 [Ui Scale](ui_scale.md)。共享排除标记 `WindowScaleExcluded` 始终从 Facade 根导出，不安装额外策略。

`splash` Feature 默认关闭，经 `splash` 模块提供显式创建独立 Splash 窗口和固定物理几何的能力；不隐式启用普通窗口缩放或窗口控件，不自动隐藏主窗口，详见 [Splash](splash.md)。

`canvas` 默认保留 Canvas 绘制、Document 与 SVG。默认关闭的 `canvas_png_export` 同时启用 `canvas` 并转发 Canvas 的 `png_export`，提供异步 PNG API 和导出系统。Gallery 显式开启；该开关与颜色主题及 Cursor 外观独立。

颜色主题默认只编译 Dark。默认关闭的 `light`、`pink_dream` 分别转发到 Theme 的对应开关；关闭时对应公开预设常量不可用；当前 Theme 由完整配色 Resource 保存，支持外部数据设置，不使用固定模式枚举。二者可以独立组合，不启用 Kamuri Cursor。Gallery 使用这三套内置预设和自身定义的 Kamuri Violet，保持四个选择项。

Cursor 的公开静态/图集动画主题始终可用，默认仍为 System，保留全部 Hint 与 Lease。库不提供 Kamuri 专用接口或 Feature；Gallery 自行用公开描述定义并选择完整角色主题，且与应用 UI 配色独立。

默认 Feature 启用全部 24 个控件模块，原有默认用法保持可用。每个控件的 Feature、公开模块和直接可选依赖一一对应：`button`、`hold_button`、`toggle_button`、`check_box`、`radio_group`、`segmented`、`tabs`、`collapsible`、`slider`、`number_input`、`progress_bar`、`splitter`、`text_field`、`combo_box`、`tooltip`、`scroll_area`、`list_view`、`tree`、`table`、`waveform`、`canvas`、`window`、`message_box`、`file_dialog`。

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
