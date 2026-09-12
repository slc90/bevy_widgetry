# Stage11：自定义 Window 与 Gallery 调整方案

## 目标范围

本次只处理两部分：

1. **自定义 Window 控件重构**
2. **Gallery 中与 Window 展示直接相关的调整**

本次不包含：

- Button 系列样式统一调整
- Theme 整体风格重新设计
- 兼容旧 API
- 双击标题栏最大化
- 运行时动态切换最小化/最大化按钮显隐

---

## 一、Window 控件重构

### 1. 对外 API 改为 BSN

移除命令式 `spawn_window()`，改为只保留 BSN 入口，不保留旧 API 兼容层。

新的主入口形式：

```rust
window(
    target_window,
    target_camera,
    controls,
    title_bar_content,
    content,
) -> impl Scene
```

其中：

- `target_window`：绑定的原生 `Window` 实体
- `target_camera`：渲染这棵 Window UI 的目标 Camera
- `controls`：窗口系统按钮配置
- `title_bar_content`：标题栏自定义内容
- `content`：窗口主体内容

约定：

- `title_bar_content` 允许传空 `bsn_list![]`
- `content` 必须提供
- 运行时允许 `commands.spawn_scene(bsn! { window(...) })` 动态创建新窗口 UI

### 2. 公开插件改为 `WindowPlugin`

不再对外暴露 `TitleBarPlugin`，只暴露 `WindowPlugin`。

`TitleBar`、`ResizeArea`、系统按钮等均作为 `window` crate 内部实现细节。

`WindowPlugin` 负责：

- title bar 相关逻辑
- resize 相关逻辑
- 系统按钮逻辑
- 内嵌 SVG 图标注册
- theme 刷新
- `WindowClosed` 后清理对应 `WindowRoot`

### 3. 内部构造全部改为 BSN

Window 整体实体树全部改为 Scene/BSN 描述：

- 删除 `TitleBar::spawn()`
- 删除 `WindowResizeArea::spawn()`
- 初始实体层级不再混用 `Commands::spawn`

内部可以拆成私有 scene 函数提升可读性，例如：

- `title_bar(...)`
- `window_content(...)`
- `window_resize_area()`
- `resize_handle(direction)`

---

## 二、Window 内部结构

内部结构固定为：

```text
WindowRoot
├── TitleBar
│   ├── TitleBarDragArea
│   ├── TitleBarContent
│   └── WindowControls
├── WindowContent
└── WindowResizeArea
```

说明：

- `TitleBarContent` 是调用方插槽，可以为空
- `WindowControls` 始终由 Window 自己提供
- `WindowContent` 是调用方主体内容插槽
- `WindowResizeArea` 覆盖窗口边缘，包含八个方向的 resize handle

---

## 三、Window 与 Camera / 原生 Window 的关系

### 1. `target_window` 与 `target_camera` 都必须显式传入

Window 不自动猜测 Camera。

原因：

- Camera 在引擎中是强语义对象
- 未来可能接入 2D、3D、子 viewport 等内容
- 同一窗口可能涉及多个 Camera，不应让 Window 控件隐式选择

因此 `window()` 必须显式接收：

- `target_window`
- `target_camera`

并把 `UiTargetCamera(target_camera)` 挂到 Window UI 根实体上。

### 2. 生命周期边界

`WindowRoot` 和 `target_window` 固定为 **1:1** 关系。

约定：

- 一个原生 `Window` 只允许对应一个 `WindowRoot`
- 多 Camera 需求属于窗口内容内部组织问题，不属于 `WindowRoot` 职责

关闭时：

- 收到 `WindowClosed(window)`
- `WindowPlugin` 查找 `target_window == window` 的 `WindowRoot`
- 清理对应整棵 Window UI 树

### 3. Camera 生命周期不由 Window 控件管理

`window()` 只**引用** `target_camera`，不**拥有**它。

因此：

- Window 关闭时，不自动清理 `target_camera`
- Camera 是否清理，由调用方根据实际业务关系决定

Gallery 的多窗口 demo 采用最简单的一对一策略：

- 每个 demo window 创建一台专用 Camera
- 收到该窗口的 `WindowClosed` 后，由 Gallery 自己清理对应 Camera

需要在函数注释里明确写清楚：

- Gallery 的一对一 Camera 生命周期只是参考示例
- 实际业务中 Camera 是否清理，取决于调用方自己的所有权关系

---

## 四、图标资源处理

Window 内部的系统按钮图标：

- minimize
- maximize
- restore
- close

全部改为编译时嵌入。

要求：

- 使用 `embedded_asset!` 注册
- 作为 `WindowPlugin` 内部资源
- `window()` 公开 API 不再暴露 `AssetServer`
- BSN 构造阶段通过 template 上下文读取 `AssetServer`
- 运行时 maximize/restore 图标切换仍可在内部系统里使用 `AssetServer`

不为本次重构修改通用 `Icon` 的公开 API。

---

## 五、样式与主题

### 1. 窗口外观

Window 需要支持：

- 圆角
- 外边框
- 外边框颜色
- 跟随 theme 的整体配色
- 三个系统按钮不跟随 theme

### 2. 圆角

只做 UI 视觉圆角，不做原生透明窗口轮廓圆角。

默认圆角半径：

- 普通状态：`8px`
- 最大化状态：`0px`

作用到：

- `WindowRoot`：四角 `8px`
- `TitleBar`：顶部两角 `8px`
- `WindowContent`：底部两角 `8px`

最大化时三者统一切为 `0px`，恢复时回到 `8px`。

### 3. Border 与分隔线

Window 自身提供外边框，不再由 Gallery 额外补画窗口边框。

标题栏与内容区之间保留底部分隔线。

### 4. Theme 接入方式

本次只做接入，不讨论整体视觉风格。

增加 Window 所需主题语义，并沿用当前 palette：

- `window_background`
- `window_border`
- `title_bar_border`

窗口背景与标题栏背景使用同一表面色，不拆成独立 title bar background token。

样式归属：

- `WindowRoot`
  - 背景：`window_background`
  - 边框：`window_border`
- `TitleBar`
  - 底边框：`title_bar_border`

系统按钮保持固定交互色，不跟随 theme。

### 5. Theme 刷新

Window 的主题刷新沿用现有 styled 控件的模式：

- 初始化时读取当前 `ThemeMode`
- 收到 `ThemeChanged` 后刷新 Window 相关颜色

刷新范围：

- `WindowRoot` 背景
- `WindowRoot` 边框
- `TitleBar` 分隔线

不处理系统按钮颜色。

---

## 六、标题栏拖动与可交互内容

保留当前拾取模型：

- `TitleBarDragArea` 绝对定位铺满标题栏底层
- `TitleBarContent` 容器本身 `Pickable::IGNORE`
- 交互控件自己正常拾取

效果应为：

- 点标题栏空白处：拖动窗口
- 点普通不可交互内容：穿透到底层拖动区，拖动窗口
- 点标题栏中的 Button / ComboBox 等交互控件：正常交互，不拖窗口
- 点系统按钮：正常触发系统按钮逻辑，不拖窗口

BSN 重构时保留同样的实体层级和拾取语义。

---

## 七、窗口行为

### 1. 最大化状态同步

最大化状态始终以 **winit 实际状态** 为准，不自己推断。

统一同步：

- maximize / restore 图标
- `WindowRoot` 圆角 `8px ↔ 0px`
- `TitleBar` 圆角 `8px ↔ 0px`
- `WindowContent` 圆角 `8px ↔ 0px`
- resize handles 启用 / 禁用

说明：

- 不支持双击标题栏最大化/还原
- 标题栏拖动仍只调用 `start_drag_move()`
- 如果平台在拖动最大化窗口时自动恢复，UI 只根据真实 `is_maximized()` 状态同步，不写额外恢复逻辑

### 2. Resize 语义

resize handles 仅在以下条件同时满足时启用：

- `Window.resizable == true`
- 当前窗口未最大化

否则：

- 不允许 resize
- 不出现 resize cursor
- 不触发 `start_drag_resize()`

### 3. 系统按钮：enabled 与 visible

#### enabled

原生 `Window.enabled_buttons` 是唯一真源：

- `minimize`
- `maximize`
- `close`

要求：

- 外部运行时修改 `Window.enabled_buttons` 后，自定义标题栏按钮自动同步可交互性
- disabled 后不可触发操作

本次不修改 disabled 视觉样式。

#### visible

额外提供构造时配置，仅控制：

- `minimize_visible`
- `maximize_visible`

约束：

- `close` 不允许隐藏
- `minimize_visible` / `maximize_visible` 只在初始化时读取
- 不支持运行时动态修改

建议配置类型：

```rust
pub struct WindowControlsConfig {
    pub minimize_visible: bool,
    pub maximize_visible: bool,
}
```

### 4. `decorations`

只要一个原生 `Window` 被这套自定义 Window 接管，就要求：

```rust
decorations = false
```

处理方式：

- `WindowRoot` 创建时一次性把目标 `Window.decorations` 设为 `false`
- 不做后续持续同步
- 外部如果主动改回 `true`，后果由调用方承担

---

## 八、错误处理策略

以下情况视为调用错误：

- `target_window` 无效
- `target_window` 没有 `Window` 组件
- `target_camera` 无效
- `target_window` 已经绑定过一个 `WindowRoot`

处理策略：

- 不 `panic!`
- 明确 `error!`
- 清理这次错误创建出来的 `WindowRoot`
- App 继续运行

创建阶段做严格校验；运行时普通 observer/system 中，生命周期竞争导致的查询失败可以安全返回。

---

## 九、插件依赖策略

`WindowPlugin` 如果依赖基础插件，例如：

- `ThemePlugin`
- `IconPlugin`

则在 `build()` 中自行确保它们已经注册。

统一规则：

- 先通过 `is_plugin_added::<T>()` 检查
- 未注册时再添加
- 避免多个高级控件重复初始化共同依赖

---

## 十、Gallery 调整

### 1. 整体展示结构

Gallery 控件展示使用左右布局：

- 左侧：sidebar
- 右侧：对应控件页面

Sidebar 增加右边框作为竖向分隔线，并补足留白：

- Sidebar 自带右边框
- Sidebar 右侧有内边距
- 右侧页面左侧有内边距

不单独增加中间 `Divider` 节点。

### 2. Window 页面

Gallery 新增一个 `Window` 页面。

页面本身只放一个：

```text
Open Window
```

点击后允许连续弹出多个窗口。

每次点击创建：

- 一个新的原生 `Window`
- 一台专用 UI Camera
- 一棵新的自定义 Window UI

用于验证：

- `window()` 的运行时创建能力
- 多实例支持
- `WindowClosed` 后 UI 清理
- Gallery 侧 Camera 清理

### 3. 弹出窗口的演示内容

弹出窗口内容保持简单：

- 标题栏内容：标题文字
- 窗口内容区：一段简单文本 + 一个 Button

用于验证：

- `title_bar_content` 可插入任意内容
- `content` 中可正常放交互控件

### 4. Gallery 主窗口标题栏内容

Gallery 主窗口的 `title_bar_content` 改为：

```text
[Logo Icon] [Widget Gallery]                [Theme ComboBox]
```

要求：

- Logo 位于标题文字左侧
- 左侧标题区域整体 `padding-left: 12px`
- Logo 与文字之间保留 `8px` 间距
- Logo 尺寸为 `16x16`
- Logo、文字垂直居中
- Logo 不紧贴标题栏左边缘

如果实际观感偏挤，优先把左边距从 `12px` 调到 `16px`。

#### Gallery Logo SVG

Logo 使用“窗口 + 左侧导航 + 右侧内容卡片”的抽象图形，体现控件展示 Gallery 的含义：

```svg
<svg width="16" height="16" viewBox="0 0 16 16" fill="none" xmlns="http://www.w3.org/2000/svg">
  <rect x="1.5" y="2" width="13" height="11.5" rx="2" stroke="currentColor" stroke-width="1.2"/>
  <path d="M1.5 5H14.5" stroke="currentColor" stroke-width="1.2"/>
  <circle cx="3.5" cy="3.6" r="0.7" fill="currentColor"/>
  <circle cx="5.5" cy="3.6" r="0.7" fill="currentColor" opacity="0.72"/>
  <circle cx="7.5" cy="3.6" r="0.7" fill="currentColor" opacity="0.44"/>
  <path d="M5.2 5.6V13.1" stroke="currentColor" stroke-width="1.2"/>
  <rect x="2.6" y="6.6" width="1.6" height="1.2" rx="0.4" fill="currentColor"/>
  <rect x="2.6" y="8.7" width="1.6" height="1.2" rx="0.4" fill="currentColor" opacity="0.78"/>
  <rect x="2.6" y="10.8" width="1.6" height="1.2" rx="0.4" fill="currentColor" opacity="0.56"/>
  <rect x="6.6" y="6.6" width="2.6" height="2.2" rx="0.5" fill="currentColor"/>
  <rect x="10.2" y="6.6" width="2.0" height="2.2" rx="0.5" fill="currentColor" opacity="0.72"/>
  <rect x="6.6" y="9.7" width="5.6" height="2.2" rx="0.5" fill="currentColor" opacity="0.5"/>
</svg>
```

使用要求：

- 颜色跟随标题栏当前前景色，使用 `currentColor`
- 作为普通 `Icon` 放入 `title_bar_content`
- 不属于 Window 内建系统按钮

---

## 十一、测试调整

Window crate 的测试改为验证新的契约：

1. `window(...)` 生成正确实体结构
2. `title_bar_content` 和 `content` 落在正确插槽
3. `WindowRoot` 正确持有 `target_window`
4. 根 UI 正确设置 `UiTargetCamera(target_camera)`
5. 创建后目标原生 `Window.decorations == false`
6. 同一个 `target_window` 重复创建第二个 `WindowRoot`
   - 输出错误
   - 第二个 Root 被清理
   - 第一个 Root 保留
7. `WindowClosed` 后对应 `WindowRoot` 整棵 UI 被清理
8. `enabled_buttons` 运行时变化会同步按钮可交互状态
9. `minimize_visible / maximize_visible` 初始化显隐正确
10. `resizable = false` 时 resize handles 不可用

不为真实 winit 最大化行为建立复杂 headless 单测，该部分通过 Gallery 真实窗口运行验证。

facade 层补公共 API 可用性验证，覆盖：

- `WindowPlugin`
- `WindowControlsConfig`
- `window(...)` 的公开入口

---

## 十二、本次明确不做的事

以下内容本次不做：

- 兼容 `spawn_window()` / `TitleBarPlugin`
- 双击标题栏最大化
- 动态修改最小化/最大化按钮显隐
- 系统按钮 disabled / hover / pressed 样式重做
- Theme 视觉风格重设计
- 原生透明圆角窗口
- 为无效 `Entity` 设计恢复机制
- 自动管理业务 Camera 生命周期
