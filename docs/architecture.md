# Bevy Widgetry Architecture

## Architecture Overview

bevy_widgetry 使用 Cargo Workspace 组织，由 Gallery 应用、顶层 Facade、各 Widget Crate、共享基础设施及测试基础设施组成。

当前 Workspace 结构：

```text
gallery/

crates/
├── bevy_widgetry/
├── core/
├── internal_log/
├── app_logging/
├── app_icon_build/
├── asset/
├── theme/
├── cursor/
├── button/
├── hold_button/
├── toggle_button/
├── check_box/
├── radio_group/
├── slider/
├── text_field/
├── combo_box/
├── tooltip/
├── scroll_area/
├── list_view/
├── tree/
├── table/
├── window/
├── message_box/
├── file_dialog/
├── waveform/
└── test_utils/
```

Button 直接依赖 Cursor 的内部忙碌光标标记，Cursor 不依赖 Button；Loading 状态和 Spinner 实现归属 Button，Disabled 理由组合继续由 Core 管理。

各 Crate 的实际依赖关系以 Cargo Manifest 为准，职责边界与依赖方向遵循 `rules/architecture.md`。

Facade 为 16 个控件提供默认全部启用的独立可选依赖与 Feature，按需使用时可关闭默认集合。Core、Theme、Cursor 与 Core 使用的内建 Asset 仍为基础依赖；控件 Feature 不取消各 Widget 自身的组合依赖，也不自动公开对应的下层 Facade 模块。

宿主日志能力由 Facade 独立且默认关闭的 `app_logging` Feature 重导出，实现仍在 app_logging crate。Gallery 显式启用该 Feature 并通过 Facade 导入，内建 Asset 和构建期 app_icon_build 保持各自的直接依赖。

## Workspace Roles

本节概述各 Workspace 成员的职责。具体功能、设计思路、行为契约和已知限制见对应的独立功能文档。

| 成员 | 职责 |
| --- | --- |
| [gallery](gallery.md) | Widgetry 的功能展示与 GUI 运行时验证应用。 |
| [bevy_widgetry](crates/bevy_widgetry.md) | 聚合并导出 Widgetry 公共 API 的顶层 Facade。 |
| [core](crates/core.md) | 提供跨 Widget 共享的 UI 基础设施、交互状态投影、组合控件输入目标元数据与组合控件支持。 |
| [internal_log](crates/internal_log.md) | 提供 Widgetry 内部统一的日志 Macro 与 Target。 |
| [app_logging](crates/app_logging.md) | 提供宿主可选的日志配置与输出基础设施。 |
| [app_icon_build](crates/app_icon_build.md) | 提供应用构建期间嵌入 Windows EXE Icon 的辅助能力。 |
| [asset](crates/asset.md) | 统一管理 Widgetry 内建 Asset、Embedded 注册及语义标识。 |
| [theme](crates/theme.md) | 提供预定义 Theme、Widget 配色及 Theme 状态管理。 |
| [cursor](crates/cursor.md) | 管理 Native Window 的 Cursor 状态及可选 Kamuri Cursor。 |
| [hold_button](crates/hold_button.md) | 提供独立长按确认、输入取消、圆形 Picking 与液面材质。 |
| [toggle_button](crates/toggle_button.md) | 独立二态按钮，复用官方 Checkbox / Checked 输入与状态，提供持续选中样式和提交后通知。 |
| [button](crates/button.md) | 提供 Button、LinkButton 控件及其交互、URL 打开与外观行为。 |
| [check_box](crates/check_box.md) | 提供 CheckBox 与 TriStateCheckBox 控件。 |
| [slider](crates/slider.md) | 提供 Slider 单值状态、输入、主题样式、尺寸配置与程序化更新。 |
| [radio_group](crates/radio_group.md) | 提供 RadioGroup 的选项组织与单选交互。 |
| [text_field](crates/text_field.md) | 提供 TextField 与 ReadOnlyTextField 控件。 |
| [combo_box](crates/combo_box.md) | 提供 ComboBox 的选项展示、选择及可选本地搜索；搜索输入组合 text_field，结果组合 list_view。 |
| [tooltip](crates/tooltip.md) | 提供 Tooltip 的内容展示与交互触发行为。 |
| [scroll_area](crates/scroll_area.md) | 提供可滚动内容区域及 Scrollbar 交互。 |
| [list_view](crates/list_view.md) | 提供稳定身份 List Model、泛型读取、逐 View 投影与独立手动排序、虚拟化列表展示及条目交互；为 Tree 提供内部结构拖放桥接。 |
| [tree](crates/tree.md) | 提供层级 Tree Model、逐 View 展开与选择，以及经校验的真实 ECS 节点拖放移动。 |
| [table](crates/table.md) | 提供 Table Model、表格布局、虚拟化单元格展示与交互。 |
| [window](crates/window.md) | 提供自定义 Window、Native Window 生命周期及多窗口支持。 |
| [message_box](crates/message_box.md) | 提供 MessageBox 的展示、操作结果与关闭行为。 |
| [file_dialog](crates/file_dialog.md) | 提供 FileDialog 的文件浏览、选择、验证及后台文件系统处理。 |
| [waveform](crates/waveform.md) | 提供 Waveform 数据处理与可视化展示。 |
| [test_utils](crates/test_utils.md) | 提供跨 Crate 共享的测试与 Benchmark 基础设施。 |

Table 直接依赖 Asset 的表头排序/筛选 SVG 语义标识，使用 Core 的 WidgetryIcon 组合独立请求按钮，并自动安装相应 Asset 与 Icon Plugin。

Waveform 的顶部时间、幅值和每屏通道数选择组合 ComboBox 与 ListView Model；标签列与波形共用 ScrollArea 的原生滚动状态，渲染 Plugin 自动安装这些 UI 能力，独立数据更新 Plugin 保持无 UI 用法。

Toggle Button 直接依赖 Core、Theme 和内部日志，不依赖 CheckBox 的固定视觉或 Button 的瞬时 Activate 行为。
