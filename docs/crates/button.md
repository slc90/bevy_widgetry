# Button

## 功能概述

提供可组合调用方自定义 BSN Children 的 Button，以及通过系统默认浏览器打开 HTTP / HTTPS URL 的独立 LinkButton。

## 当前设计思路

采用 Bevy Button 的 Activation 机制，Widgetry 负责外观与内容配色。普通 Button 的内容、尺寸与布局由调用方提供；LinkButton 自行维护 URL 文字，调用方可设置根 Node 的布局。

## 主要功能

- 通过 `WidgetryButton` Scene 创建控件并组合 Text、Icon 或其他 Children。
- 通过官方 `Activate` Observer 响应用户触发。
- 配色随 Hover、Pressed、Disabled、Theme 与颜色覆盖变化。
- Foreground 颜色可传递到 Widgetry 管理的内容。
- `WidgetryLinkButton` 自行创建内部文字，始终显示 URL 原文；支持自动换行，不截断或显示省略号，不接受自定义标签或 Children。
- LinkButton 使用独立四态主题颜色，不带普通按钮背景和边框。Normal 无下划线，Hovered / Pressed 有下划线，Disabled 无下划线。

## 行为与状态契约

状态优先级为 Disabled → Pressed → Hover → Normal。`InteractionDisabled` 拒绝用户 Activation，移除禁用后可以继续响应交互。外观变化不应重置调用方提供的 `Node` Layout Patch 或 Children。

两种控件均由 `WidgetryButtonPlugin` 安装，复用官方 Button、Widgetry Cursor、字体、前景色传播及 Disabled 继承。

LinkButton 默认填满父容器可用宽度，使长 URL 在确定的宽度内换行；调用方可通过根 Node 的 `width` 设置所需宽度。

LinkButton 构造示例：

```rust
bsn! { @WidgetryLinkButton { @url: {"https://bevy.org".to_owned()} } }
```

URL 保存在内部持久 Component 中，Props 仅用于初始化。通过 `WidgetryLinkButton::url(world, entity)` 查询原文；`set_url_in_world(world, entity, url)` 同步更新 URL 和内部文字，返回 `Result<bool, BevyError>`；`set_url(commands, entity, url)` 在命令执行时校验和更新。同值设置返回 `Ok(false)` 且不写入状态。程序化修改不触发浏览器或 `Activate`，URL 属于展示与跳转配置，不另发状态变化事件。

`WidgetryLinkButtonColorOverrides` 提供 `normal`、`hovered`、`pressed`、`disabled` 四个可选颜色；支持 Props `@colors` 初始化，以及 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 运行期 API。未覆盖的颜色随当前主题变化。

## 边界情况与错误处理

Theme 或颜色覆盖变更后，按控件当前状态解析外观。普通 Press/Release 及被禁用的输入不应凭空产生业务操作；构造时由 BSN 和对应 Plugin 提供必要基础能力。

LinkButton 仅接受包含主机的完整 `http://` / `https://` 地址（Scheme 不区分大小写），允许 Unicode、查询参数、Fragment、合法端口及 IPv6。拒绝空白、控制字符、反斜杠、非法地址或非 HTTP(S) Scheme。URL 原文不会被解析器规范化后替换。无效 URL、无效 API 目标及非法覆盖颜色记录错误并以 `Severity::Error` 返回；修改失败保留旧配置和文字。BSN 校验失败通过 Scene 错误路径传播，延迟构造应使用 Widgetry Scene 错误适配。

LinkButton 内部 `Activate` Observer 在非 Disabled 时将当前 URL 交给 Windows `ShellExecuteExW` 的 `open` 操作。系统请求失败仅通过 `widgetry_warn!` 记录原因，不发送失败事件、不弹窗，也不监测网页加载结果。

## 已知限制

- 普通 Button 的业务动作由应用 Observer 实现，标签和 Icon 由外部内容提供。
- LinkButton 的内部文字和 Hierarchy 由控件维护，调用方不应直接修改或替换。
- Bevy 官方 Button 激活契约仍然适用。

