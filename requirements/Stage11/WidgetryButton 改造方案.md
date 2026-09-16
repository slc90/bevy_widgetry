# WidgetryButton 改造方案

## 目标

Button 不再维护自定义 headless 行为。

行为完全建立在 Bevy 0.19 官方 `ui_widgets::Button` 上；Widgetry 只提供自己的视觉样式、主题响应与 BSN Scene。

正式构造方式统一为 BSN：

```rust
bsn! {
    @WidgetryButton
}
```

不再维护旧式 `commands.spawn(WidgetryButton)` 构造方式。

---

## 1. 删除 LongPress

彻底删除当前学习阶段遗留的 LongPress 能力：

- 删除 `crates/button/src/headless.rs`
- 删除 `crates/button/tests/long_press.rs`
- 删除：

  - `LongPressButton`
  - `LongPressEvent`
  - `LongPressPlugin`
  - `LongPressPending`

- 删除所有相关 re-export、测试和公开 API 验证
- 修改 button crate 顶层文档，不再描述“长按行为”

这次不将 LongPress 泛化为 gesture，也不保留 feature 或兼容层。

原因仅是当前没有需求，不否定 `LongPressButton` 这种 Component 扩展方式本身。

---

## 2. StyledButton 更名

统一改名：

```rust
StyledButton       -> WidgetryButton
StyledButtonPlugin -> WidgetryButtonPlugin
```

`WidgetryButton` 表示：

> 使用 Widgetry 默认视觉样式的 Bevy headless Button。

---

## 3. WidgetryButton 改为 SceneComponent

`WidgetryButton` 使用 Bevy 0.19 `SceneComponent`。

它本身无配置字段、无 Props。

可采用简单的 unit SceneComponent：

```rust
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryButton;
```

关联 Scene 负责一次性提供完整默认组成，不再使用 `#[require]`。

Scene 包含：

```text
Button
Hovered(false)
TabIndex(-1)
Node
BackgroundColor
BorderColor
Propagate<ForegroundColor>
```

其中官方 `Button` 自己要求的 accessibility 等组件继续由 Bevy 自身负责。

### 默认 Node

```rust
Node {
    min_height: px(32),
    padding: UiRect::axes(px(12), px(6)),
    border: UiRect::all(px(1)),
    border_radius: BorderRadius::all(px(4)),
    ..default()
}
```

不设置：

```text
width
min_width
height
flex_direction
align_items
justify_content
row_gap / column_gap
```

WidgetryButton 只提供按钮外壳的默认几何，不规定 children 的排布。

外部通过 BSN patch 自己决定内容布局。

---

## 4. Focus 策略

继续保留：

```rust
TabIndex(-1)
```

WidgetryButton 默认不参与 Tab 键盘导航。

当前不实现 focused 视觉状态，也不增加 focus outline。

支持的视觉状态只有：

```text
Normal
Hovered
Pressed
Disabled
```

优先级继续保持：

```text
Disabled > Pressed > Hovered > Normal
```

---

## 5. 状态样式

继续由运行期系统根据官方状态组件更新：

```text
BackgroundColor
BorderColor
Propagate<ForegroundColor>
```

颜色继续来自现有 `ColorTheme`：

```text
control_background
control_background_hovered
control_background_pressed
control_background_disabled

control_border
control_border_hovered
control_border_pressed
control_border_disabled

foreground
foreground_disabled
```

不新增：

- ButtonVariant
- Primary / Danger / Ghost
- Focus 配色
- Button 私有颜色配置
- ButtonProps

### BSN patch 边界

几何和布局组件可以正常通过 BSN patch：

```text
Node.width
Node.height
padding
排列
gap
```

但主题驱动的：

```text
BackgroundColor
BorderColor
ForegroundColor
```

属于 WidgetryButton 运行期样式系统的输出。

单纯通过 BSN patch 覆盖这些颜色并不是持久定制方式，因为 hover / pressed / disabled / ThemeChanged 时会重新计算。

当前没有自定义状态配色需求，因此不增加颜色 override API。

---

## 6. WidgetryButtonPlugin

保留当前样式依赖：

```text
ForegroundColorPlugin
WidgetryFontPlugin
ThemePlugin
```

并补上：

```rust
ButtonPlugin
```

如果官方 `ButtonPlugin` 尚未注册，则由 `WidgetryButtonPlugin` 注册。

这样 WidgetryButtonPlugin 不只保证视觉系统存在，也保证其依赖的官方 Button 行为 observers 已经装配。

不自己重新实现任何 Button pointer / activate 行为。

---

## 7. Button 测试迁移

删除所有 LongPress 测试。

将：

```text
crates/button/tests/styled_button.rs
```

改为与新名称一致的 Button 测试文件，例如：

```text
crates/button/tests/widgetry_button.rs
```

实际控件创建统一改为：

```rust
spawn_scene(bsn! {
    @WidgetryButton
})
```

不再通过：

```rust
spawn(WidgetryButton)
```

测试继续覆盖：

- Normal
- Hovered
- Pressed
- Disabled
- 状态优先级
- Pressed 移除恢复
- Disabled 移除恢复
- Theme Light / Dark 切换
- foreground propagation

另外增加 Scene 构造测试，确认 `@WidgetryButton` 展开后具备：

```text
WidgetryButton
Button
Hovered(false)
TabIndex(-1)
Node 默认几何
BackgroundColor
BorderColor
Propagate<ForegroundColor>
```

并验证 `WidgetryButtonPlugin` 已确保 `ButtonPlugin` 注册。

不重新测试 Bevy 官方 Button 内部的 click / Activate 状态机。

---

## 8. facade 公共 API

修改：

```text
crates/bevy_widgetry/tests/public_api.rs
```

删除 LongPress 公开 API 验证。

将：

```text
StyledButton
StyledButtonPlugin
```

替换为：

```text
WidgetryButton
WidgetryButtonPlugin
```

公共构造测试通过：

```rust
spawn_scene(bsn! {
    @WidgetryButton
})
```

验证 facade 用户无需访问 button 内部模块即可使用完整 WidgetryButton Scene。

---

## 9. MessageBox 迁移

MessageBox 是当前 Button 的直接消费者，必须一起修改。

将：

```rust
StyledButton
StyledButtonPlugin
```

迁移为：

```rust
WidgetryButton
WidgetryButtonPlugin
```

`MessageBoxPlugin` 继续确保 Button 插件已经注册，但目标改成：

```rust
WidgetryButtonPlugin
```

结果按钮从旧式：

```rust
template(|_| Ok(StyledButton))
```

改为 BSN Scene：

```rust
@WidgetryButton
```

MessageBox 自己已有的按钮尺寸 patch 保留，例如：

```text
min_width = 84
height = 36
内容居中
```

MessageBox 测试中的普通正文 Button 同样改成 `@WidgetryButton`。

---

## 10. Gallery Sidebar

Gallery 导航按钮全部迁移到：

```rust
@WidgetryButton
```

Sidebar 调整为：

```rust
Node {
    width: px(176),
    border: UiRect::right(px(1)),
    padding: UiRect::all(px(16)),
    row_gap: px(10),
    flex_shrink: 0.0,
    flex_direction: FlexDirection::Column,
    align_items: AlignItems::Center,
}
```

导航按钮：

```rust
Node {
    width: percent(100),
    height: px(40),
    align_items: AlignItems::Center,
    justify_content: JustifyContent::Center,
    ..
}
```

效果：

- 四周都有留白
- 导航按钮不再贴边
- 每个按钮之间保持统一间距
- 文字居中

导航文字作为纯视觉 child，使用 `Pickable::IGNORE`，避免 children 抢走 Button 的 pointer target。

Gallery 主程序中的插件注册同步改为：

```rust
WidgetryButtonPlugin
```

---

## 11. Gallery Button Page

Button Page 用两行展示完整内容组合。

### 第一行：Normal

```text
[ Text ]
[ Icon + Text ]
[ Text + Icon ]
[ Icon ]
```

### 第二行：Disabled

```text
[ Text ]
[ Icon + Text ]
[ Text + Icon ]
[ Icon ]
```

Disabled 四个实例全部附加：

```rust
InteractionDisabled
```

这样可以直接对比：

- Button background
- Button border
- Text foreground
- Icon foreground

在 normal / disabled 两种状态下是否都正确变化。

Hovered 和 Pressed 不制作静态假状态实例。

Normal 行直接通过真实鼠标 hover / press 展示这两个状态。

### 页面布局

页面整体纵向：

```text
Normal
四个 Button

Disabled
四个 Button
```

两行 Button 都使用：

```rust
Node {
    flex_direction: FlexDirection::Row,
    align_items: AlignItems::Center,
    column_gap: px(12),
    ..
}
```

两组之间约 `16px` 间距。

### Button 内容布局

WidgetryButton 本身不负责 children 排布，因此 demo 自己 patch。

文字按钮：

```text
内容居中
```

Icon + Text：

```text
Row
Center
column_gap = 6px
```

Text + Icon：

```text
Row
Center
column_gap = 6px
```

Icon only：

```text
32 × 32
padding 覆盖为适合图标按钮的值
内容居中
```

这同时自然展示 BSN 对默认 WidgetryButton Node 的 patch 能力，不另外增加“Custom Button”示例。

---

## 12. Gallery Button SVG

新增一个 Button demo 专用 SVG，例如：

```text
gallery/src/assets/icons/button_star.svg
```

使用：

```text
16 × 16
currentColor
简单清晰的 star 图形
```

不写死最终显示颜色。

由 Widgetry `Icon` 的 ForegroundColor 继承机制决定 normal / disabled 状态颜色。

扩展：

```rust
GalleryIcon
```

增加：

```rust
ButtonStar
```

并由 `GalleryAssetPlugin` 使用 `embedded_asset!` 注册。

Button Page 的四种 Icon 示例统一使用：

```rust
@Icon {
    @path: { GalleryIcon::ButtonStar.path() },
    @max_size: { Some(UVec2::new(16, 16)) },
}
```

不提供显式 `color`，以便验证 `ForegroundColor` 传播。

Icon root和 Text child 都使用 `Pickable::IGNORE`，保证点击目标仍然是 Button。

---

## 13. 不做的内容

本次明确不增加：

- LongPress
- DoubleClick
- Focus 样式
- ButtonVariant
- Primary / Danger / Ghost Button
- 自定义颜色 Props
- Button caption Props
- Button Icon Props
- 固定宽度
- 全局内容布局策略
- 动画
- 阴影
- 通用 gesture 层

现有 `requirements` 中记录早期学习阶段的 LongPress / StyledButton 历史内容不作为本次代码改造的执行目标；本次只修改当前代码和直接受影响的测试 / Gallery / MessageBox。

---

## 完成后的使用模型

普通文字按钮：

```rust
bsn! {
    @WidgetryButton
    Node {
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..
    }
    Children [
        (Text("Button") Pickable::IGNORE)
    ]
}
```

Icon + Text：

```rust
bsn! {
    @WidgetryButton
    Node {
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        column_gap: px(6),
        ..
    }
    Children [
        (
            @Icon {
                @path: {icon_path},
                @max_size: {Some(UVec2::new(16, 16))},
            }
            Pickable::IGNORE
        ),
        (Text("Button") Pickable::IGNORE),
    ]
}
```

Button 行为来自 Bevy。

Button 默认视觉来自 Widgetry。

具体内容与局部布局由调用方通过 BSN 组合和 patch。
