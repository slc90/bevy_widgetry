# Button

## 功能概述

提供通过 `@children` / `@content` 组合调用方 BSN 内容、支持业务显式 Loading 的 Button，以及通过系统默认浏览器打开 HTTP / HTTPS URL 的独立 LinkButton。

## 当前设计思路

普通 Button 通过 `WidgetryButtonHeadless` 提供官方 `Activate` 事件、主键输入、键盘输入和无障碍 Button 角色，默认 `WidgetryButton` 组合此能力与外观。LinkButton 保留 Bevy 原生 Button。普通 Button 的内容、尺寸与布局由调用方提供；LinkButton 自行维护 URL 文字，调用方可设置根 Node 的布局。

## 主要功能

- 通过 `WidgetryButton` Scene 创建控件并组合 Text、Icon 或其他 Children。
- 通过官方 `Activate` Observer 响应用户触发。
- 配色随 Hover、Pressed、Disabled、Theme 与颜色覆盖变化。
- Foreground 颜色可传递到 Widgetry 管理的内容。
- `WidgetryLinkButton` 自行创建内部文字，始终显示 URL 原文；支持自动换行，不截断或显示省略号，不接受自定义标签或 Children。
- LinkButton 使用独立四态主题颜色，不带普通按钮背景和边框。Normal 无下划线，Hovered / Pressed 有下划线，Disabled 无下划线。

## 行为与状态契约

普通 Button 的视觉优先级为 Loading → Disabled → Pressed → Hover → Normal。Loading 使用当前 Normal 配色，并在中心显示两层圆环；约 300° 前景弧按 1.2 秒一圈顺时针旋转，底环保持固定。`InteractionDisabled` 拒绝用户 Activation，移除禁用后可以继续响应交互。外观变化不应重置调用方提供的 `Node` Layout Patch 或 Children。

`WidgetryButton` 的根 Entity 保留 Headless、Activate 目标、背景边框和外部尺寸。调用方通过 `@children: Some(Box<dyn SceneList>)` 传入内容，通过 `@content: Some(Box<dyn Scene>)` 配置内部 Content 容器的布局（例如 `column_gap`、`flex_direction`、`justify_content`）；根 Node 的尺寸、Padding 等 Patch 独立保留。不要在根上追加业务 Children 绕过 Content 容器。

```rust
bsn! {
    @WidgetryButton {
        @children: {Some(Box::new(bsn_list! { Text("Save") WidgetryText }) as Box<dyn SceneList>)},
        @content: {Some(Box::new(bsn! { Node { column_gap: px(6) } }) as Box<dyn Scene>)},
    }
    Node { width: px(160), height: px(40) }
    on(save)
}
```

Loading 默认 `false`，不提供初始化 Prop。业务使用 `WidgetryButton::is_loading` 查询，以 `set_loading_in_world` 同步更新，或用 `set_loading` 在 Commands 实际执行时提交。合法同值返回 `Ok(false)`，实际改变返回 `Ok(true)`；无效目标返回 Error，排队错误交由宿主 Handler。`WidgetryButtonLoadingChanged { entity, loading }` 仅在状态实际改变并提交后发出。业务自行负责任务启动、成功、失败及取消后的 Loading 收尾。

普通 Button 的 loading 和颜色 World API 返回 `ButtonFailure = WidgetryFailure<ButtonError>`。`InvalidTarget` 保留无效根，`MissingPart` 区分 Headless、LoadingState、Content、Spinner 及其必需组件，`ManagedColors` 保留内部颜色的 Owner；托管颜色输出失败保留缺失组件列表和真实 Query source。Core disabled/颜色失败保留原 CoreError、origin、source 与首次回溯，仅附 Button 操作、根、颜色状态与字段上下文。Command、Scene Template 和 Theme Observer 的边界显式转为 Error Severity，不在传播时重记源头或 handled。

`WidgetryButtonColorOverrides` 的 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 支持四态背景、边框及前景色覆盖。非法颜色、托管颜色写入或结构缺失保留原配置；初始覆盖校验经 Scene 失败清理路径传播。Loading 在真实状态变化前预检 Core disabled 目标，缺少运行资源时保留内容、Spinner、光标与 loading 状态；合法同值请求仍直接返回未变化。正常 Spinner 资产等待不构造错误。

Loading 以独立内部理由参与有效 Disabled，阻止正常鼠标、Enter 和 Space 生成新的 Activate；结束时只撤销自身理由，保留本地、祖先及其他内部禁用。直接由代码 trigger 的 Activate 不经过输入禁用检查；Commands 生效前也不提供任务并发保护。Loading 不主动修改 Focus 或默认 `TabIndex(-1)`。

Loading 隐藏自有 Content 包装层，并压缩该层的渲染变换以覆盖显式 Visible 子元素；不改 Layout 占位、业务实体、子元素 Visibility 或原有变换，结束后恢复包装层。Spinner 为无 Picking 的绝对居中层，遮罩在运行时生成并共享，颜色随 Theme / Button 覆盖变化，非 Loading 时停止旋转。Loading 光标使用 Wait 并优先于 Disabled 的 NotAllowed，最终图像由独立 Cursor Theme 决定。LinkButton 不提供 Loading。

两种控件均由 `WidgetryButtonPlugin` 安装，复用 Widgetry Cursor、字体、前景色传播及 Disabled 继承。

LinkButton 默认填满父容器可用宽度，使长 URL 在确定的宽度内换行；调用方可通过根 Node 的 `width` 设置所需宽度。

LinkButton 构造示例：

```rust
bsn! { @WidgetryLinkButton { @url: {"https://bevy.org".to_owned()} } }
```

URL 保存在内部持久 Component 中，Props 仅用于初始化。通过 `WidgetryLinkButton::url(world, entity)` 查询原文；`set_url_in_world(world, entity, url)` 同步更新 URL 和内部文字，返回 `Result<bool, LinkButtonFailure>`；`set_url(commands, entity, url)` 在命令执行时校验和更新。同值设置返回 `Ok(false)` 且不写入状态。程序化修改不触发浏览器或 `Activate`，URL 属于展示与跳转配置，不另发状态变化事件。

`WidgetryLinkButtonColorOverrides` 提供 `normal`、`hovered`、`pressed`、`disabled` 四个可选颜色；支持 Props `@colors` 初始化，以及 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 运行期 API。未覆盖的颜色随当前主题变化。

## 边界情况与错误处理

Theme 或颜色覆盖变更后，按控件当前状态解析外观。普通 Press/Release 及被禁用的输入不应凭空产生业务操作；构造时由 BSN 和对应 Plugin 提供必要基础能力。

LinkButton 仅接受包含主机的完整 `http://` / `https://` 地址（Scheme 不区分大小写），允许 Unicode、查询参数、Fragment、合法端口及 IPv6。拒绝空白、控制字符、反斜杠、非法地址或非 HTTP(S) Scheme。URL 原文不会被解析器规范化后替换。LinkButton 的 URL 与颜色 World API 返回 `LinkButtonFailure = WidgetryFailure<LinkButtonError>`。`InvalidUrl` 保留根、输入原文与 `LinkUrlRejection`：不安全字符、非完整地址、不支持的 Scheme、缺少主机、原始 `url::ParseError` 或 `SyntaxViolation`；`InvalidTarget` 与 `MissingPart` 区分非法目标和内部结构损坏。非法覆盖颜色保留 CoreError 的真实 source、原始 origin 和颜色字段。修改失败保留旧配置和文字；正常同值静默。Command、Scene Template 和 Theme 查询边界显式转为 `Severity::Error`，保留同一诊断，不在传播时重复源头日志。BSN 校验失败通过 Scene 错误路径传播并清理新建根，延迟构造应使用 Widgetry Scene 错误适配。

LinkButton 内部 `Activate` Observer 在非 Disabled 时将当前 URL 交给 Windows `ShellExecuteExW` 的 `open` 操作。原生失败以 `OpenBrowser` 保留请求根、URL 及 `winsafe::co::ERROR`，源头记录一次 ERROR；激活 Owner 安全放弃当前请求后，以同一错误 ID 记录处理 DEBUG，并释放诊断，不重试、不发送失败事件、不弹窗，也不监测网页加载结果。Command 执行时再次检查目标与 Disabled；已删除或禁用视为正常取消，不调用 opener、不产生错误。

## 已知限制

- 普通 Button 的业务动作由应用 Observer 实现，标签和 Icon 由外部内容提供。
- LinkButton 的内部文字和 Hierarchy 由控件维护，调用方不应直接修改或替换。
- Bevy 官方 Button 激活契约仍然适用。


## 自定义外观的基础入口

`WidgetryButtonHeadless` 从 Button crate 及 Facade `button` 模块公开，可直接使用 `bsn! { @WidgetryButtonHeadless Node { width: px(160), height: px(40) } on(save) }`。安装 `WidgetryButtonPlugin` 后，根即为 Picking、Focus、`Activate` 和状态查询目标；Headless 不创建 Children、背景、边框、默认内容或 Loading。外部样式读取 Bevy `Hovered` / `Pressed` 与 Core `WidgetryEffectiveDisabled`。

Headless 不附加原生 `bevy::ui_widgets::Button`，防止其未过滤的副键处理器产生重复或错误激活。只有主键完成有效点击才激活，取消或拖动后不激活，副键不改变主键 Pressed。根处截断 Pointer Press/Click/Release/Cancel 与 Drag 传播，避免内嵌按钮触发表格选择或列拖动；同一根上的消费者 Observer 仍可处理副键；Enter / Space 在获得焦点后按下时激活，忽略重复按键与松开。保留官方 Focus / TabIndex 模型，默认 `TabIndex(-1)` 可点击聚焦，Tab 遍历由宿主有效配置决定。不要在 Headless 根额外挂原生 Button。显式、祖先与内部 Disabled 阻止真实输入，代码直接触发 `Activate` 仍由调用者负责。
