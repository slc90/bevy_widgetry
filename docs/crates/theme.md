# Theme

## 功能概述

提供 Widgetry 的完整配色数据、内置预设，以及各 Widget / 组合部件的完整颜色数据与全局主题切换能力。默认只编译 Dark；Light、Pink Dream、Kamuri Violet 分别通过默认关闭的 `light`、`pink_dream`、`kamuri_violet` Feature 开启。

## 当前设计思路

Theme 是统一的颜色数据和切换通知源，不直接承担所有 Widget 的样式更新。当前 `WidgetryTheme` Resource 保存完整配色，是唯一主题状态。每个消费者从它和自身状态选择颜色输出；外部可复制任一预设、修改字段，再通过相同 Setter 应用。Theme 仅包含颜色，不管理布局、尺寸或动画。

每个可选主题控制整套公开常量、各控件的配色常量与公共色值。关闭时这些 API 不存在，不在运行时回退 Dark；`WidgetryTheme` 的完整字段结构保持稳定。Facade 转发三个同名 Feature，Gallery 显式启用全部主题，三个开关可以独立组合，且与 `kamuri_cursor` 无关。Cargo Feature 在依赖图中累加，精简构建需使用独立调用方验证。

## 主要功能

- Dark、Light、Pink Dream、Kamuri Violet 四套内置预设，以及外部定义的完整配色。
- 为 Text、Icon、Button、LinkButton、CheckBox、ComboBox、Tooltip、ScrollArea、ListView、Tree、Table、Window、Dialog、Waveform 等提供 Colors Type。
- `WidgetryTheme::link_button` 提供独立的 Normal、Hovered、Pressed、Disabled 前景颜色，四套主题均有完整定义。
- `WidgetryTheme::text_field.hint_warning` 提供常显提示的黄色警告语义，四套主题分别配置；其显示不因 TextField Disabled 而变灰。
- NumberInput 组合复用 `text_field` 的可编辑/只读外框与文本配色，以及 `button` 的局部交互配色；四种主题均同步更新整个组合。
- 提供 World 和 Commands 主题切换入口，以及 `WidgetryThemeChanged` 通知。
- `WidgetryTheme::progress_bar` 为四套主题提供无交互态的 Track、Fill、Border 配色，ProgressBar 支持逐字段局部覆盖。
- `WidgetryTheme::slider` 为四套主题定义 Normal / Hovered / Pressed / Disabled 的 Track、Fill、Thumb 和焦点环颜色，Slider 局部覆盖可按状态覆盖各字段。
- `WidgetryTheme::splitter` 为四套主题定义 Normal / Hovered / Dragging / Disabled 的 Handle 线条颜色，Splitter 支持逐状态局部覆盖。
- `WidgetryTheme::segmented` 提供容器 Normal / Focused / Disabled 背景与边框、Option Checked / Unchecked 下 Normal / Hovered / Disabled 的背景与前景。
- `WidgetryTheme::tabs` 提供标签栏、内容框与分隔线颜色，Header Selected / Unselected 的 Normal / Hovered / Pressed / Disabled 背景、边框、前景及焦点边框；四套主题均保留选中禁用的区别。
- Waveform 配色包含起始 Channel Palette 与 Disabled Palette。

## 行为与状态契约

默认 Dark。`WidgetryTheme::set_in_world(world, theme)` 返回是否发生实际变化；`set(commands, theme)` 排队执行。Setter 校验所有颜色后原子替换完整 Resource，再发送不携带模式枚举的 `WidgetryThemeChanged`；Observer 从 Resource 读取已提交数据。合法同值设置不标脏、不发送通知。Commands Queue 仅在实际执行时生效；Style Consumer 根据通知或其自身生命周期刷新对应输出。

## 边界情况与错误处理

`set_in_world` 在缺少 `WidgetryTheme` 时返回 `ThemeFailure` / `ThemeError::MissingTheme`；非有限颜色分量或超出 [0,1] 的 alpha 以 `InvalidColor { field, color }` 返回，保留字段路径且不部分修改配色。Waveform 的调色板至少包含两色，循环相邻色须不同，非法数据以 `InvalidPalette` 拒绝。Commands 在实际执行时校验，失败显式按 Error Severity 传播；诊断保留源头身份和 Backtrace，源头只记录一次 ERROR。合法同值切换返回 `Ok(false)`，不产生日志；成功切换仍先提交资源、通知并 flush。消费者未安装或尚未构造的 UI 不由 Theme Plugin 自动补建；Theme 切换不会修改各控件的业务 State。

## 已知限制

- 不包含运行时主题编辑器；调用方通过完整数据设置主题。
- Gallery 自行维护四个预设名称和持久化配置；收到目录外完整配色时清空选择，不将其误标为内置预设。
- Colors 定义并不直接保证每个未安装的 Widget 会自动更新 UI。
- 本 crate 的预设配色、全部预设转移通过源码 Unit Test 验证；`tests/runtime_theme.rs` 覆盖公开自定义配色、连续切换、queued 与同值静默、提交后通知及非法嵌套颜色的原子拒绝；`test_utils::theme` 单测补充同步与 queued 的领域类型、source/origin、原始回溯、源头一次 ERROR、无处理 DEBUG、执行时缺资源与真实宿主 Error Severity，以及辅助切换的正常无日志。


HoldButton 使用独立的 `hold_button: WidgetryHoldButtonColors`：Normal / Hovered / Pressed / Disabled 各含 background、liquid、wave、border、foreground，另有 success_border。四套 Theme 均提供浅底、深色液体与固定前景色。
