# ToggleButton

ToggleButton 是保存选中状态的二态按钮，适合开关工具、切换显示选项。它与普通 Button 不同：一次操作改变的是持续的选中状态，而不只是发出瞬时激活。

```rust
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy_widgetry::toggle_button::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryToggleButtonPlugin);
```

## 二态选择

### 创建带初值的按钮

```rust
bsn! {
    @WidgetryToggleButton { @selected: true }
    Children [Text("显示网格") WidgetryText]
    on(changed)
}
```

`selected` 只负责初始化。选中后，按钮继续使用选中态配色，不会在松开鼠标时自动恢复未选中。

### 查询和更新选中状态

默认按钮与自定义外观都通过 Headless 的状态入口操作：

```rust
fn enable_grid(world: &mut World, root: Entity) -> Result {
    let selected = WidgetryToggleButtonHeadless::selected(world, root)
        .map_err(|error| error.into_bevy_error())?;
    if !selected {
        WidgetryToggleButtonHeadless::set_selected_in_world(world, root, true)
            .map_err(|error| error.into_bevy_error())?;
    }
    Ok(())
}
```

需要排队时，调用 `WidgetryToggleButtonHeadless::set_selected(&mut commands, root, true)`。同步入口返回状态是否真正改变；同值操作不重复通知。

### 接收提交后的变化

```rust
fn changed(event: On<WidgetryToggleButtonChanged>) {
    info!(root = ?event.entity, selected = event.selected, "工具开关已改变");
}
```

用户交互和公开 Setter 都先提交 `Checked`，再发送专用 Changed 事件。官方 `ValueChange<bool>` 是输入请求，不用于代替提交后的通知。直接增删原生 `Checked` 可以改变状态，但不会补发专用 Changed。

## 交互禁用

### 暂停用户切换

```rust
commands.entity(root).insert(InteractionDisabled);
// 程序化修改仍可通过公开 Setter 进行。
WidgetryToggleButtonHeadless::set_selected(&mut commands, root, false);
```

禁用不等于取消选中。一个已选中且禁用的按钮仍保持选中状态，只是不接受用户切换。鼠标、Enter、Space 与焦点沿用相应的二态输入行为。

## 默认外观

### 提供文字或复合内容

```rust
bsn! {
    @WidgetryToggleButton
    Node { padding: UiRect::all(px(10)) }
    Children [Text("测量模式") WidgetryText]
}
```

默认外观区分选中/未选中与 Normal、Hovered、Pressed、Disabled 的组合状态。它不带 CheckBox 固定方框或勾选图形，内容可以按应用需要组合。

## 自定义外观

### 复用状态和输入

```rust
app.add_plugins(WidgetryToggleButtonHeadlessPlugin);

let scene = bsn! {
    @WidgetryToggleButtonHeadless { @selected: false }
    Node { height: px(48), padding: UiRect::all(px(8)) }
    Children [Text("标签") Pickable::IGNORE]
    on(changed)
};
```

只安装 Headless 时，不需要默认按钮标识、背景、边框或默认颜色状态。自己的 Style 从 `Checked` 和交互状态生成外观；不要保存第二份长期选中布尔值。
