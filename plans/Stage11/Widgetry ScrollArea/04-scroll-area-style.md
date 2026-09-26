# 实现 WidgetryScrollArea Scene、Content contract 与 scrollbar visual style

## 目标

在已经稳定的 headless contract 上完成 WidgetryScrollArea 的公开 BSN SceneComponent、一次性 Props、默认 hierarchy、Content patch 规则、scrollbar track/thumb visual style 与完整 WidgetryScrollAreaPlugin。

## WidgetryScrollArea 与 Props

公开 SceneComponent：

```rust
#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryScrollAreaProps)]
pub struct WidgetryScrollArea;
```

Props 是唯一公开 construction config 入口：

```rust
pub struct WidgetryScrollAreaProps {
    pub axis: ScrollAxis,
    pub scrollbar_visibility: ScrollbarVisibility,
    pub scrollbar_thickness: f32,
    pub content: Option<Box<dyn Scene>>,
    pub children: Option<Box<dyn SceneList>>,
}
```

Default：

```text
axis                  = Vertical
scrollbar_visibility  = Auto / Auto
scrollbar_thickness   = 12.0
content               = None
children              = None
```

Props: Default 是 Bevy 0.19.1 SceneComponent 的硬要求。content / children 都是 one-shot Scene 输入，不使用 factory；空 ScrollArea 合法，不 panic。

axis / visibility / thickness 初始化 private ScrollAreaConfig；content / children 在 Scene 展开完成后结束生命周期，不保存同义 prop state。

## Root

Root 是 Widgetry 对外 identity 和 focus target：

- WidgetryScrollArea。
- TabIndex(-1)。
- 默认 Display::Grid，并使用上一方案确定的 2×2 Grid tracks。
- 不设置默认 width / height；容器尺寸完全由调用方 Bevy layout 决定。

Root Node 仍允许调用方在 @WidgetryScrollArea 后 patch。调用方如果覆盖 display / grid_template_* 等结构字段破坏内部默认 Grid，由调用方负责；库不建立额外防御层。

## Viewport

Viewport 是 Root direct child：

- 公开 WidgetryScrollAreaViewport marker。
- 官方 bevy_ui_widgets::ScrollArea。
- ScrollPosition。
- overflow 由 axis 内部保护：Vertical = Clip/Scroll，Horizontal = Scroll/Clip，Both = Scroll/Scroll。
- scrollbar_width 保持 0。
- 占据 Grid column 1 / row 1。

Viewport 的功能 Node 字段属于 headless contract，不通过公开 content props 让调用方覆盖。

## Content

Content 是 Viewport 的唯一 direct child，并带 crate-private ScrollAreaContent marker。

默认布局方向：

```text
Vertical   → FlexDirection::Column
Horizontal → FlexDirection::Row
Both       → FlexDirection::Column
```

这是 convenience default，用户 content Scene 可以覆盖 flex direction、wrap、padding、gap、align、justify、display / grid 等普通内容 layout 字段。

### protected sizing contract

用户 content patch 之后必须再应用 internal invariant patch：

```rust
Node {
    width: Val::Auto,
    height: Val::Auto,
    min_width: percent(100),
    min_height: percent(100),
    max_width: Val::Auto,
    max_height: Val::Auto,
    flex_shrink: 0.0,
    overflow: Overflow::visible(),
    ..
}
```

组合顺序固定：

```text
Content default Scene
→ user content Scene
→ internal invariant patch LAST
```

因此通过公开 content construction path，用户无法覆盖这些 protected Node fields。它不是 ECS 安全边界；调用方如果之后自己找到 Content entity 并直接修改 component，出现异常由调用方负责。

语义目标：

```text
Vertical   → Content width >= Viewport width，高度随 children 自然增长
Horizontal → Content height >= Viewport height，宽度随 children 自然增长
Both       → 两轴都至少填满 Viewport，也允许任一轴被 children 撑大
```

如果实现时发现 Bevy/Taffy 对 Auto + percent(100) 的真实 layout 无法满足这一已确认语义，不得静默改成另一套 contract；应先用最小 layout test / Gallery 证明问题，再回到设计层调整。

## Scrollbar entity

axis 决定实际创建哪些官方 Scrollbar：

```text
Vertical   → V
Horizontal → H
Both       → H + V
```

即使 policy = Hidden，只要 axis 存在，对应 Scrollbar entity 仍属于固定结构，只是 Node.display = None。

每个 Scrollbar：

- target 指向 Viewport。
- orientation 使用官方 ControlOrientation。
- min_thumb_length = 24.0 logical px，v1 不公开配置，直接写官方 Scrollbar.min_thumb_length，不保存重复 state。
- H.height / V.width 使用 private config 中的 scrollbar_thickness。
- Grid placement 使用上一方案确定的位置；无 Corner entity。

## visual style

Track 默认透明，不画 background / border。

Thumb 使用官方 ScrollbarThumb + Hovered + ScrollbarDragState：

```text
normal   → ThemeMode.colors().control_border
hover    → control_border_hovered
dragging → control_border_pressed
```

优先级 dragging > hover > normal。ThemeChanged 时按当前 Hovered / dragging state 立即重新解析颜色。

默认 border radius = 6 logical px。thumb 的功能 size / position 继续完全由官方 ScrollbarPlugin 计算，Widgetry 不复制 geometry 算法。

不为 track / thumb 增加额外 AccessibilityNode；v1 accessibility 只沿用官方 headless 意图，不自行扩展。

## WidgetryScrollAreaPlugin

公开 plugin 负责完整装配，按现有项目习惯用 is_plugin_added 避免重复：

- Bevy ScrollAreaPlugin。
- Bevy ScrollbarPlugin。
- Bevy TabNavigationPlugin。
- Widgetry ThemePlugin。
- ScrollArea crate 自己的 headless systems / observers 与 style systems。

不安装 InputFocusPlugin / InputDispatchPlugin；正常应用由 DefaultPlugins 提供，文档说明真实 focus / keyboard 输入依赖这些官方基础设施。

TabNavigationPlugin 在这里是必要依赖：ScrollArea root 的 TabIndex(-1) 需要它提供 pointer press → AcquireFocus，才能实现“点击普通内容后 nearest ScrollArea root 获得 focus”的既定语义。

## visibility

正式 public API：

- WidgetryScrollArea。
- WidgetryScrollAreaPlugin。
- WidgetryScrollAreaProps。
- WidgetryScrollAreaViewport。
- ScrollAxis。
- ScrollbarPolicy。
- ScrollbarVisibility。
- WidgetryScrollIntoView。

crate 内跨 module 使用：ScrollAreaConfig、ScrollAreaContent、Auto solve state 等按需要 pub(crate)。只在单 module 使用的 helper / state 保持 private。

DEFAULT_SCROLLBAR_THICKNESS = 12.0 与 DEFAULT_MIN_THUMB_LENGTH = 24.0 保持内部常量，不作为 public API。

## 自动化测试

按 TDD 覆盖 Widgetry 自己的 Scene / style contract：

- Props default 与空 content / children 合法。
- axis 创建正确数量和 orientation 的 Scrollbar。
- Root 不带默认固定尺寸，Grid placement 正确，无 Corner。
- Viewport overflow 与 scrollbar_width 受内部 contract 控制。
- Content default direction 正确；用户 content 可覆盖允许字段，而 protected sizing fields 在最终 Scene 中仍由 invariant patch 胜出。
- scrollbar thickness 从 props 初始化到 functional geometry。
- min_thumb_length 固定为 24 且没有额外重复 state。
- thumb normal / hover / dragging / theme refresh 配色。
- public WidgetryScrollAreaViewport 与 ScrollPosition 可以从外部 query。

不要重复测试 Bevy Scrollbar 已经保证的 thumb 比例算法。

## 预期产出

得到一个可以直接通过 @WidgetryScrollArea 构造的完整 styled Widget，headless functional geometry 与 theme visual style 分工清晰，对外 API 与现有 Widgetry 习惯一致。

## 与前后方案的关系

依赖前两个 ScrollArea headless 方案。完成后 ScrollArea crate 自身功能已经闭环；下一方案只做 workspace facade / public API / architecture 最终接入，不再改变 Widget 行为。
