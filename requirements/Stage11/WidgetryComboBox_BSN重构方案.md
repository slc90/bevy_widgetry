# WidgetryComboBox BSN 化重构方案

## 目标

把现有 ComboBox 从 `headless + styled` 双层构造方式重构为单一的 BSN `SceneComponent` 控件，并保留现有 ComboBox 的主要交互语义。

这次只处理 ComboBox 及其直接受影响的 Gallery / facade / 资产 / 测试，不提前抽 `WidgetryListItem`、`WidgetryPopup` 或 Editable ComboBox。

## 一、公开 API

最终只保留：

```rust
WidgetryComboBox
WidgetryComboBoxProps
WidgetryComboBoxOptionFactory
WidgetryComboBoxPlugin
```

删除旧 API：

```rust
ComboBox
ComboBoxPlugin
StyledComboBoxPlugin
SetComboBoxSelected
spawn_headless_combo_box
spawn_styled_combo_box
```

不保留 deprecated alias，也不增加 `combo_box_option(...)` helper。

### WidgetryComboBoxProps

```rust
pub struct WidgetryComboBoxProps {
    pub options: Vec<WidgetryComboBoxOptionFactory>,
}
```

要求：

- `Default` 为 `options: Vec::new()`，用于满足 `SceneComponent`。
- 实际 `scene(props)` 构造时要求 options 非空；空列表直接 assert。
- 不提供 `selected` 字段。
- 初始化时始终默认选中 index `0`。
- 如需其他初始项，构造后调用 `WidgetryComboBox::set_selected(...)`。

### WidgetryComboBoxOptionFactory

使用可复用 factory，而不是一次性 `SceneList`：

```rust
#[derive(Clone)]
pub struct WidgetryComboBoxOptionFactory(
    Arc<dyn Fn() -> Box<dyn SceneList> + Send + Sync>,
);
```

提供：

```rust
pub fn new<S, F>(factory: F) -> Self
where
    S: SceneList + 'static,
    F: Fn() -> S + Send + Sync + 'static;

pub(crate) fn build(&self) -> Box<dyn SceneList>;
```

factory 必须可以重复调用，因为同一 option 内容需要：

1. 构造 Popup 中的 option；
2. 构造当前选中项在 Field 中的副本；
3. 之后 selection 变化时重新构造 Field 内容。

如果 closure 捕获拥有所有权的数据，调用方应自行 clone / 以可重复调用方式使用。

## 二、内部文件结构

删除原先按 `headless.rs / style.rs` 堆叠的组织方式，改成按 ComboBox 组成部分拆分：

```text
crates/combo_box/src/
├── lib.rs
├── combo_box.rs
├── field.rs
├── popup.rs
└── option.rs
```

职责建议：

### `combo_box.rs`

放整个控件级别的内容：

- `WidgetryComboBox`
- `WidgetryComboBoxProps`
- `WidgetryComboBoxOptionFactory`
- `ComboBoxOptions`
- 整体 `scene(props)` 装配
- `WidgetryComboBox::set_selected()`
- 用户选择后的 ComboBox 级 value 语义
- selection 唯一性 helper
- root `InteractionDisabled` 的控件级语义

### `field.rs`

- `ComboBoxField`
- `ComboBoxFieldContent`
- `ComboBoxDropdownIcon`
- Field 的 `@WidgetryButton` 结构与布局覆盖
- `Added<Selected>` 后重建 FieldContent
- root disabled 到 Field 的镜像同步
- Popup visibility 到上下箭头的同步

### `popup.rs`

- `ComboBoxPopup`
- Popup 结构与样式
- open / close helper
- outside click 关闭
- reselect 当前 option 时关闭

### `option.rs`

- `ComboBoxOption { index }`
- `ListItem` wrapper
- option row 的容器样式
- hover / selected / disabled 样式
- theme 刷新

### `lib.rs`

保持薄层：

- `mod` 声明
- 只 re-export 四个 public API
- `WidgetryComboBoxPlugin` 的依赖和 systems 装配

## 三、场景结构

整体结构：

```text
WidgetryComboBox + ComboBoxOptions
├── ComboBoxField + WidgetryButton
│   ├── ComboBoxFieldContent
│   │   └── selected option factory.build()
│   └── ComboBoxDropdownIcon + Icon
└── ComboBoxPopup + ListBox + Visibility::Hidden
    ├── ComboBoxOption(0) + ListItem + Selected
    │   └── option[0].build()
    ├── ComboBoxOption(1) + ListItem
    │   └── option[1].build()
    └── ...
```

Popup option 必须继续保留 `ListItem` wrapper；arbitrary option content 作为它的 children。

初始化时：

- index 0 带 `Selected`；
- Popup 初始 Hidden；
- `ComboBoxOptions(options)` 把完整 factory Vec 留在 root，供运行期重建 Field 使用。

## 四、selection 行为

### 用户选择不同 option

保持现有逻辑：

```text
ListBox ValueChange<Entity>
→ 验证属于当前 ComboBox
→ 保证唯一 Selected
→ 关闭 Popup
→ root 发 ValueChange<usize>
```

`ValueChange<usize>` 只表示用户导致的真实 value change。

### 用户再次点击当前 option

补上当前缺失的 `ReselectListRow` 处理：

```text
ReselectListRow
→ 只关闭 Popup
→ Selected 不变
→ Field 不重建
→ 不发 ValueChange<usize>
```

### 程序化选择

公开 API：

```rust
impl WidgetryComboBox {
    pub fn set_selected(
        commands: &mut Commands,
        entity: Entity,
        selected: usize,
    );
}
```

内部使用轻量 `commands.queue(move |world: &mut World| { ... })`，不再保留任何 `SetComboBoxSelected` / 私有 event。

queued command 只负责：

- 验证 root / index；
- 找目标 option；
- 移除旧 `Selected`；
- 给目标插入 `Selected`。

不要在该 closure 中重建 Field scene。

语义：

- entity 无效：no-op；
- index 越界：no-op；
- 已经选中目标 index：no-op；
- root disabled 时仍允许程序化选择；
- 成功后不发送 `ValueChange<usize>`。

## 五、Field 内容同步

Field 内容始终从真实 `Selected` 状态派生，而不是从 `ValueChange` 派生。

使用普通 `Update` system：

```text
Query<(Entity, &ComboBoxOption), Added<Selected>>
→ 找所属 WidgetryComboBox
→ 找 ComboBoxFieldContent
→ 递归 despawn FieldContent 现有 children
→ options[index].build()
→ 把新 SceneList 作为 FieldContent children spawn
```

要求：

- `ComboBoxFieldContent` 自己保留，只清它的 children；
- children 递归 despawn；
- 不做 diff，不保留旧 Field 副本内部状态；
- Field 只是 selection 的派生展示。

## 六、InteractionDisabled

对外规则不变：

> 复合控件的 `InteractionDisabled` 只要求调用方挂在 `WidgetryComboBox` root 上。

root 是唯一 authoritative state。

由于 Field 复用完整 `WidgetryButton`，内部需要把 disabled 状态镜像到 Field；这只是内部适配，不改变 public 语义。

采用三个普通 system，不用 lifecycle observer：

1. root `Added<InteractionDisabled>` → 给已有 Field 加 `InteractionDisabled`；
2. root 的 `InteractionDisabled` 被移除 → 从已有 Field 移除；
3. `Added<ComboBoxField>` → 查询 root 当前状态，初始化 Field disabled。

三个 system 放在 `PreUpdate`。

这样 `WidgetryButton` 自己在 `Update` 中可在同一帧看到 Field 上新增/移除的 disabled，并刷新样式。

注意：

- ComboBox 自身行为仍以 root disabled 为准；
- root 新增 disabled 时继续关闭已打开 Popup；
- 不递归向所有 descendants 传播 disabled；
- 只显式同步给内部那些真正作为独立控件工作的子控件；当前只有 Field。

## 七、Field 样式

Field 直接复用：

```text
ComboBoxField + @WidgetryButton
```

删除旧 ComboBox 自己维护的 Field 颜色状态系统。

`WidgetryButton` 负责：

- normal
- hovered
- pressed
- disabled
- BackgroundColor
- BorderColor
- ForegroundColor propagation

旧的 Popup-open `active` 配色不再保留；ComboBox 不再使用 `control_background_active / control_border_active`，但这次不删除 theme 中这些 token。

ComboBox 只覆盖 WidgetryButton 的 Field 尺寸与布局，保留当前 ComboBox 的既有 Field 尺寸/间距，不强行采用 WidgetryButton 的默认几何值。

## 八、Dropdown Icon

去掉文本 `v`，改成一对内建 SVG：

```text
chevron_down.svg
chevron_up.svg
```

要求：

- 相同尺寸 / viewBox，避免切换时布局抖动；
- 加入 `bevy_widgetry_asset` 的 embedded assets；
- `BuiltinIcon` 增加对应语义项；
- Field 中使用现有 `Icon` 控件；
- Popup Hidden → down；Popup Visible → up；
- 不额外保存 `open` 状态；箭头完全从 Popup `Visibility` 派生；
- 使用现有 `Icon::set_svg(...)` 做运行时 SVG 替换。

## 九、Popup 样式

基本保留现有 Popup：

- absolute positioning；
- 宽度 / 相对 Field 位置保持现状；
- `GlobalZIndex` 保持现状；
- popup background / border 保持现状；
- 不引入 popover 定位系统；
- 不新增 shadow / 额外 padding 改造。

仅补：

```rust
border_radius: BorderRadius::all(px(4))
```

## 十、Option 样式

Option 内容完全由 factory 返回的 arbitrary children 决定；ComboBox 不规定 Text / Icon / 具体内容布局。

但 `ComboBoxOption + ListItem` wrapper 仍保留一层轻量 row style，用于 ListBox 交互反馈：

- 行布局 / 宽度 / 现有高度与 padding；
- normal background；
- hovered background；
- selected background；
- disabled 配色；
- 默认 ForegroundColor propagation；
- 新增 `4px` 圆角。

不要创建 `WidgetryListItem`；等以后真正实现 ListBox 时再抽。

## 十一、Plugin 与依赖

统一只保留：

```rust
WidgetryComboBoxPlugin
```

调用方只需要注册它，不应再知道旧的 `ComboBoxPlugin + StyledComboBoxPlugin` 双插件结构。

Plugin 负责确保 ComboBox 直接依赖的控件 / 设施已注册，例如：

- `WidgetryButtonPlugin`；
- `IconPlugin`；
- ComboBox 需要的 theme / foreground / ListBox / Button 行为插件；
- 内建 chevron 资产对应的 asset plugin。

按项目现有模式检查是否已注册，避免重复添加。

ComboBox crate 的 Cargo 依赖需要补齐直接使用的 `button` / `asset` crate。

新的 ComboBox 本身不再创建固定 Text，因此不再为了 ComboBox 自动安装默认字体设施；option 里的 Text 属于调用方内容。删除旧 `StyledComboBoxPlugin` 隐式提供字体的相关约束 / 测试。

## 十二、Gallery 调整

### 页面统一留白

当前 PageHost 只有左侧 padding。改为页面内容统一四周留白：

- page content 使用统一 `16px` padding；
- 移除 / 调整现有 PageHost 的左侧 `16px`，避免左边叠成双倍。

### ComboBox 页面

改为一行四个 demo，不加标题：

```text
Text ComboBox
Icon + Text ComboBox
Icon-only ComboBox
Disabled ComboBox
```

要求：

- 四个放同一 Row；
- `column_gap` 约 16px；
- 不 wrap；
- 四个 ComboBox 使用一致宽度，避免视觉参差；
- Disabled demo 只在 `WidgetryComboBox` root 上挂 `InteractionDisabled`；
- Text / Icon+Text / Icon-only 都使用新的 `WidgetryComboBoxOptionFactory`；
- Gallery demo 图标继续走现有 `Icon` 体系，可复用或补最小量 Gallery 专用 SVG。

### 标题栏主题 ComboBox

`gallery/src/main.rs` 中现有 theme ComboBox 也必须迁移：

- `spawn_styled_combo_box` → `@WidgetryComboBox`；
- `SetComboBoxSelected` → `WidgetryComboBox::set_selected(...)`；
- `StyledComboBoxPlugin` → `WidgetryComboBoxPlugin`；
- `ValueChange<usize>` 监听语义保持不变。

## 十三、直接影响文件

实施时至少检查并迁移这些位置：

```text
crates/combo_box/src/*
crates/combo_box/Cargo.toml
crates/combo_box/tests/*

crates/asset/src/lib.rs
crates/asset/src/assets/icons/*

crates/bevy_widgetry/tests/public_api.rs

gallery/src/gallery.rs
gallery/src/main.rs
gallery/src/pages/combo_box.rs
gallery/src/assets.rs / gallery/src/assets/icons/*（如 demo 需要新 icon）
```

facade `crates/bevy_widgetry/src/lib.rs` 当前是 wildcard re-export；确认新 ComboBox public surface 能通过它正常导出即可。

## 十四、测试 / 验收重点

至少覆盖：

1. `@WidgetryComboBox` 能通过 BSN 正确构造完整层级；
2. option 支持 arbitrary SceneList 内容；
3. 初始 index 0 为唯一 `Selected`；
4. 初始 FieldContent 能从 option 0 factory 构造；
5. 用户选择不同项：更新 Selected、重建 Field、关闭 Popup、发 `ValueChange<usize>`；
6. 再次点击当前项：关闭 Popup，但不发 value change；
7. `set_selected()`：成功更新 selection + Field，但不发 user `ValueChange`；
8. `set_selected()` 越界 / 无效 entity / 相同 index 均 no-op；
9. disabled root 阻止用户交互，但不阻止程序化 `set_selected()`；
10. root disabled 增删会正确镜像到 Field `WidgetryButton`；
11. disabled 时如果 Popup 已打开会关闭；
12. Popup visibility 改变时 chevron 能在 up/down SVG 间切换；
13. Popup / Option 4px 圆角和现有 hover / selected / disabled 样式正常；
14. facade public API 测试更新为四个新 public 类型；
15. Gallery 四个 ComboBox demo 能同排显示，page content 四周有统一留白；
16. Gallery 标题栏主题选择仍能正确切换 Dark / Light。

## 非目标

这次不要顺手做：

- `WidgetryListItem` / `WidgetryListBox`；
- 独立 `WidgetryPopup`；
- Editable ComboBox / EditText ComboBox；
- 通用递归 disabled propagation；
- Popup popover / 智能定位；
- Theme active token 清理；
- 为旧 ComboBox API 提供兼容层。
