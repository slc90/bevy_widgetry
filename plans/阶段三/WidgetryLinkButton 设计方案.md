# WidgetryLinkButton 设计方案

## 目标与范围

为 Bevy Widgetry 增加独立的 `WidgetryLinkButton`，用于通过系统默认浏览器打开 HTTP / HTTPS URL。

仅实现第三阶段 Button 规划中的 Link 功能，不改变现有 `WidgetryButton` 的行为。

## 一、控件设计

### 1. 构造与内容

- 采用独立的 `WidgetryLinkButton` SceneComponent。
- BSN 构造时只需提供 `@url`。
- 不提供 `@label`，也不支持调用方自定义 Children。
- 控件自行创建并维护内部 Text，**显示内容始终与当前 URL 原文一致**。
- URL 可以自动换行，但不能截断或显示省略号。
- URL Text 不参与国际化，不能独立于 URL 修改。

### 2. URL 管理

- 创建时必须校验 URL，只允许格式合法的 HTTP / HTTPS 地址。
- URL 保存在持续存在的 ECS Component 中，而不是运行期保留 Props。
- 提供符合仓库现有规范的公开查询与程序化修改 API，支持同步 World 操作和 Commands 延迟操作。
- 修改成功后，同时更新显示文字和实际跳转地址。
- 设置相同 URL 不产生多余更新。
- URL 无效时返回 `BevyError`，保留原来的有效 URL 和文字。
- 通过 BSN 创建时的校验失败，遵循仓库现有 Scene 错误传播规范。

### 3. 交互

- 复用 Bevy 官方 Button 的交互机制。
- 由控件内部注册 `Activate` Observer。
- 使用者不需要自行实现点击跳转。
- 点击时通过 Windows 系统接口请求默认浏览器打开当前 URL。
- Disabled 状态下不能激活。
- 程序化修改 URL 不会触发浏览器打开。

### 4. 外观

- 采用超链接外观，无普通按钮的填充背景和边框。
- Normal：显示主题定义的链接颜色，不带下划线。
- Hovered：显示 Hover 颜色，带下划线。
- Pressed：显示 Pressed 颜色，保持下划线。
- Disabled：显示禁用颜色，无下划线。
- 复用现有主题、字体、Cursor、Disabled 继承与颜色覆盖机制。
- 为 LinkButton 提供独立主题配色，覆盖 Light、Dark、PinkDream、KamuriViolet 四套主题。

### 5. 错误处理

- 创建和程序化修改时，非法 URL 通过公开 API 返回错误。
- 请求系统打开浏览器失败时，按仓库日志规则使用 `widgetry_warn!` 记录原因。
- 不额外发送失败事件，不弹出 MessageBox。
- 不监测浏览器启动后的网页加载结果。

## 二、Gallery 展示

继续使用现有 Button 页面，不新增导航入口，也不修改原有演示。

新增 LinkButton 区域：

- 正常链接示例。
- Disabled 链接示例。
- 程序化切换 URL 示例：初始地址为 `https://www.baidu.com`；点击普通按钮，在该地址与 `https://www.google.com` 之间切换。
- 切换后立即更新 LinkButton 的显示文字与目标地址，只有点击链接才启动浏览器。

同时完成关联展示：

- 在 Theme Colors 中展示 LinkButton 的主题颜色。
- 在 Overrides 中增加 LinkButton 默认配色与局部覆盖的对比示例。
- 演示标题、按钮标签和说明加入中英文国际化；URL 本身不翻译。
- 使用现有 Gallery 页面生命周期与日志规范。

## 三、集成与验证

涉及的主要位置：

- `crates/button`：LinkButton、URL 管理、点击行为与颜色覆盖 API。
- `crates/theme`：独立 LinkButton 主题配色及四套主题。
- `gallery/src/pages/button.rs`：LinkButton 演示。
- `gallery/src/color_showcase`：主题颜色目录与覆盖示例。
- `gallery/src/i18n`：演示文案。
- `docs/crates/button.md`、`docs/gallery.md`：实现后的功能文档同步。

验证重点包括 URL 合法性、失败时状态不变、文字与地址一致、程序化切换、Disabled 行为、四套主题、颜色覆盖以及 Hover / Pressed 下划线表现。涉及系统浏览器的测试仅验证系统调用请求及失败处理，不把网页是否成功加载作为控件验收条件。

所有实现仍须遵循仓库适用的工程规则及既有系统约定。
