# Stage 5 总结：Color Theme

## 本阶段目标

为 `bevy_widgetry` 增加一套简单、固定、可运行时切换的颜色 Theme 系统。

本阶段 Theme **只负责颜色**，不负责尺寸、padding、字体、布局、Skin 或其它样式属性。

当前支持：

- Dark Theme
- Light Theme
- 运行时切换 Theme
- 已存在控件在切换后立即刷新
- 新创建控件使用当前 Theme
- Theme 切换不依赖每帧轮询

---

## 一、最终设计

### 1. `ColorTheme`

Theme 的颜色数据统一收敛到 `ColorTheme`：

```rust
pub struct ColorTheme {
    pub foreground: Color,
    pub foreground_disabled: Color,

    pub control_background: Color,
    pub control_background_hovered: Color,
    pub control_background_pressed: Color,
    pub control_background_active: Color,
    pub control_background_disabled: Color,

    pub control_border: Color,
    pub control_border_hovered: Color,
    pub control_border_pressed: Color,
    pub control_border_active: Color,
    pub control_border_disabled: Color,

    pub popup_background: Color,
    pub popup_border: Color,

    pub item_background_hovered: Color,
    pub item_background_selected: Color,
}
```

这里采用的是**语义颜色**，而不是控件专属颜色。

例如 ComboBox 的 Open 状态映射到：

```text
control_background_active
control_border_active
```

没有使用 `combo_box_open_*` 之类的专属字段。

这样以后其它控件如果也有 active 状态，可以复用同一组颜色语义。

---

## 二、固定 Palette

Theme 数据本身是编译期固定的：

```rust
pub const DARK_THEME: ColorTheme = ...;
pub const LIGHT_THEME: ColorTheme = ...;
```

颜色使用：

```rust
Color::srgb_u8(...)
```

定义。

### Dark Theme

```text
foreground                  #EBEDF0
foreground_disabled         #7E848E

control_background          #303237
control_background_hovered  #3A3D43
control_background_pressed  #282B30
control_background_active   #2B3848
control_background_disabled #25272B

control_border              #535760
control_border_hovered      #5E99FF
control_border_pressed      #4486F5
control_border_active       #4486F5
control_border_disabled     #3B3E45

popup_background            #25272B
popup_border                #494D55

item_background_hovered     #343D49
item_background_selected    #2A4A73
```

### Light Theme

```text
foreground                  #23262B
foreground_disabled         #8E949E

control_background          #F7F8FA
control_background_hovered  #ECEFF4
control_background_pressed  #DEE4EC
control_background_active   #E6EEF8
control_background_disabled #EEF0F3

control_border              #B7BCC5
control_border_hovered      #558EEB
control_border_pressed      #3A79DA
control_border_active       #3A79DA
control_border_disabled     #CFD3DA

popup_background            #FFFFFF
popup_border                #C6CBD3

item_background_hovered     #EEF3F9
item_background_selected    #DCEAFF
```

---

## 三、`ThemeMode`

当前运行中的 Theme 用 Resource 表示：

```rust
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ThemeMode {
    Light,

    #[default]
    Dark,
}
```

默认 Theme 为 Dark。

同时提供：

```rust
impl ThemeMode {
    pub const fn colors(self) -> &'static ColorTheme {
        match self {
            Self::Light => &LIGHT_THEME,
            Self::Dark => &DARK_THEME,
        }
    }
}
```

关系变成：

```text
ThemeMode
    ↓
colors()
    ↓
&'static ColorTheme
    ↓
LIGHT_THEME / DARK_THEME
```

Theme 数据固定，运行时只切换 `ThemeMode`。

---

## 四、`ThemeChanged`

Theme 切换通知使用普通全局 `Event`：

```rust
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThemeChanged {
    pub mode: ThemeMode,
}
```

不使用 `EntityEvent`，因为 Theme 改变是全局 UI 状态变化，而不是针对某个 Entity。

调用方负责：

```rust
*theme_mode = new_mode;
commands.trigger(ThemeChanged { mode: new_mode });
```

或者测试中：

```rust
*world.resource_mut::<ThemeMode>() = new_mode;
world.trigger(ThemeChanged { mode: new_mode });
```

本阶段没有增加 `set_theme()` helper，也没有增加额外 Theme SystemParam。

原因是 Theme 切换属于低频操作，未来计划由标题栏里的 Theme ComboBox 触发，没有必要提前抽象。

---

## 五、Observer 方案

Theme 切换采用 Observer，而不是每帧检查：

```rust
Res<ThemeMode>::is_changed()
```

结构为：

```text
ThemeMode 修改
    ↓
ThemeChanged
    ↓
Styled Widget Observer
    ↓
读取当前控件 ECS 状态
    ↓
重新 resolve
    ↓
写回颜色组件
```

优点：

- 不增加每帧 Theme 检测 system
- Theme 切换时立即刷新
- 控件状态不会被重置
- Theme 切换逻辑与普通状态刷新逻辑复用

需要注意：

```rust
world.trigger(...)
```

会立即执行 Observer。

而：

```rust
commands.trigger(...)
```

会等 Commands 被 apply 后再真正 trigger。

但一旦 trigger 真正发生，Observer 仍然是同步执行。

对于当前 UI 规模，Theme 切换只涉及少量控件和颜色组件更新，不需要考虑分帧处理。

---

## 六、`ThemePlugin`

增加了一个很薄的 ThemePlugin：

```rust
pub struct ThemePlugin;
```

职责只有：

```rust
app.init_resource::<ThemeMode>();
```

如果应用提前插入了自己的 `ThemeMode`，`init_resource` 不会覆盖它。

Styled Button 和 Styled ComboBox Plugin 会在需要时自动添加 ThemePlugin。

---

# Button 改造

## 七、Button Style Resolver

Button 原来只解析 BackgroundColor。

现在改为完整解析：

```rust
struct ButtonStyle {
    background: Color,
    border: Color,
    foreground: Color,
}
```

resolver 接受：

```rust
fn resolve_button_style(
    colors: &ColorTheme,
    hovered: bool,
    pressed: bool,
    disabled: bool,
) -> ButtonStyle
```

状态优先级保持不变：

```text
Disabled > Pressed > Hovered > Default
```

颜色映射：

```text
Default
→ control_background
→ control_border
→ foreground

Hovered
→ control_background_hovered
→ control_border_hovered
→ foreground

Pressed
→ control_background_pressed
→ control_border_pressed
→ foreground

Disabled
→ control_background_disabled
→ control_border_disabled
→ foreground_disabled
```

---

## 八、Button 刷新

Button 状态变化现在统一更新：

```text
BackgroundColor
BorderColor
Propagate<ForegroundColor>
```

通过统一的 `apply_button_style` 应用 resolver 结果。

ThemeChanged Observer 同样复用这一逻辑。

因此：

```text
状态变化
和
Theme 切换
```

不会各自维护一套不同的样式判断。

同时处理了 `Added<StyledButton>`，保证新控件会根据当前 Theme resolve。

---

# ComboBox 改造

## 九、Field Style

ComboBox Field resolver 改为接受 `&ColorTheme`。

优先级仍然是：

```text
Disabled > Open > Pressed > Hovered > Default
```

Open 状态映射为：

```text
control_background_active
control_border_active
```

---

## 十、Option Style

Option resolver 同样改为读取 Theme。

优先级保持：

```text
Disabled > Hovered > Selected > Default
```

映射：

```text
Default
→ popup_background
→ foreground

Selected
→ item_background_selected
→ foreground

Hovered
→ item_background_hovered
→ foreground

Disabled
→ control_background_disabled
→ foreground_disabled
```

---

## 十一、Popup Style

原来的：

```rust
POPUP_BACKGROUND
POPUP_BORDER
```

硬编码常量被删除。

Popup 现在使用：

```text
popup_background
popup_border
```

同时增加 `PopupStyles`，让 Theme 切换时 Popup 也能刷新。

这是本阶段的重要补充，因为原来的 Popup 颜色只在 setup 时写入一次。

---

## 十二、ComboBox Theme Observer

ComboBox Theme Observer 会刷新：

```text
Field
Options
Popup
```

为了避免多个 mutable Query 的访问冲突，使用：

```rust
ParamSet
```

按顺序调用：

```text
FieldStyles
OptionStyles
PopupStyles
```

已有的：

```text
refresh()
refresh_root()
```

结构得到了保留和复用。

---

# 测试调整

## 十三、删除 `EXPECTED_*`

原有 Button 和 ComboBox integration tests 中复制 palette 的：

```text
EXPECTED_DEFAULT_BG
EXPECTED_HOVERED_BG
EXPECTED_FIELD_OPEN_BG
EXPECTED_POPUP_BG
...
```

全部删除。

真实 integration test 直接使用：

```rust
DARK_THEME.control_background
LIGHT_THEME.control_background
...
```

这样 Theme 颜色只维护一份。

---

## 十四、Resolver Tests 使用 `TEST_THEME`

resolver 单元测试不使用真实 Dark / Light palette。

改为定义独立：

```rust
const TEST_THEME: ColorTheme = ...
```

其中每个字段都使用不同颜色值。

这样测试真正验证的是：

```text
当前状态
    ↓
是否映射到正确 semantic slot
```

而不是验证某个颜色常量等于它自己。

覆盖了：

### Button

```text
Default
Hovered
Pressed > Hovered
Disabled > all
```

### ComboBox Field

```text
Default
Hovered
Pressed > Hovered
Open > Pressed
Disabled > all
```

### ComboBox Option

```text
Default
Selected
Hovered > Selected
Disabled > all
```

---

## 十五、Theme Core Tests

测试了：

```text
ThemeMode::default() == Dark

ThemeMode::Dark.colors() == DARK_THEME

ThemeMode::Light.colors() == LIGHT_THEME
```

同时验证 ThemePlugin 不会覆盖应用提前插入的 ThemeMode。

---

## 十六、Theme 切换 Integration Tests

测试中定义了 test-only helper：

```rust
fn switch_theme(app: &mut App, mode: ThemeMode) {
    *app.world_mut().resource_mut::<ThemeMode>() = mode;
    app.world_mut().trigger(ThemeChanged { mode });
}
```

它不是生产 API。

### Button

覆盖：

- 新 Button 使用当前 Theme
- Hovered / Pressed / Disabled 状态在 Theme 切换后保持
- Theme 切换后颜色立即变化

### ComboBox

覆盖：

- 新 ComboBox 使用当前 Theme
- Open Field 在切换后仍然保持 Active style
- Selected Option 保持 Selected
- Hovered Option 保持 Hovered
- Popup Background / Border 更新
- Disabled 状态保持
- Foreground 也随 Theme 更新

关键点：

```rust
world.trigger(ThemeChanged { ... });
```

之后直接断言颜色。

没有额外调用：

```rust
app.update();
```

因此测试明确锁定了 Observer 的即时刷新行为。

---

# Public API

Theme API 从 core 导出：

```rust
ColorTheme
DARK_THEME
LIGHT_THEME
ThemeChanged
ThemeMode
ThemePlugin
```

同时主 crate：

```rust
bevy_widgetry::style
```

也重新导出了这些类型。

因此应用层可以统一通过：

```rust
bevy_widgetry::style::ThemeMode
bevy_widgetry::style::DARK_THEME
...
```

访问 Theme 基础设施。

---

# 本阶段没有实现的内容

以下内容明确留到以后：

```text
自定义 Theme
运行时修改 palette
任意 token 注册
CSS-like cascade
Skin
尺寸/布局 Theme
Theme 动画
set_theme helper
自动 Theme ComboBox
SVG Icon
TextField
截图视觉回归
```

---

# Stage 5 最终结果

Stage5 最终完成了：

```text
固定 Light / Dark Palette
        ↓
ThemeMode Resource
        ↓
ThemeMode::colors()
        ↓
ThemeChanged Event
        ↓
Observer
        ↓
Styled Button / ComboBox 重新 resolve 当前状态
        ↓
Background / Border / Foreground 即时刷新
```

整个实现没有引入：

```text
每帧 Theme polling
额外 token → color system
复杂 Theme framework
提前抽象的 set_theme API
```

并且保留了现有 Button / ComboBox 的状态优先级和 Headless 行为。

Stage5 到这里完成。
