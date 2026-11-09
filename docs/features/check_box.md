# CheckBox

CheckBox 提供二态勾选和三态勾选。二态适合开关选项；三态增加 Indeterminate，用于表达一组项目中只有部分已选中。

```rust
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::check_box::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryCheckBoxPlugin);
```

## 二态勾选

### 创建并指定初值

```rust
bsn! {
    @WidgetryCheckBox
    Checked
    Children [Text("接收更新通知") WidgetryText]
    on(binary_changed)
}

fn binary_changed(event: On<ValueChange<bool>>) {
    info!(root = ?event.source, requested = event.value, "收到勾选输入请求");
}
```

附加 `Checked` 表示初始选中；省略表示未选中。组件使用原生二态状态，不需要另存一个布尔值。

这里的官方 `ValueChange<bool>` 是输入请求，由控件行为提交到 Checked，不是独立的“状态已提交”通知。读取最终状态时查询已经提交的 Checked，不依赖同一事件中不同 Observer 的执行先后。

### 程序化更新

```rust
commands.entity(root).insert(Checked);
commands.entity(other_root).remove::<Checked>();
```

这些操作遵循 Bevy 组件语义，不自动模拟一次用户输入事件。业务需要区别用户操作和程序同步时，不要通过手动触发输入请求来代替修改状态。

## 三态勾选

### 创建部分选中控件

三态组件使用 `WidgetryCheckState`。构造后通过受控入口设置所需状态：

```rust
let scene = bsn! {
    @WidgetryTriStateCheckbox
    Children [Text("选择全部") WidgetryText]
    on(tri_changed)
};

// root 为该三态控件的根实体。
WidgetryTriStateCheckboxHeadless::set_state(
    &mut commands,
    root,
    WidgetryCheckState::Indeterminate,
);

fn tri_changed(event: On<ValueChange<WidgetryCheckState>>) {
    info!(root = ?event.source, state = ?event.value, "三态状态变化");
}
```

Indeterminate 表示独立的第三种状态，不等同于“未选中且不可用”。子项目的全选、全不选和部分选中如何汇总，由业务决定；控件不会自动扫描任意业务子树。

### 保持两套状态入口分开

二态以 `Checked` 表达状态，三态通过自己的状态 API 提交和循环。不要把给三态根增删 Checked 当成三态状态更新，也不要根据 Indicator 当前显示的形状反推业务状态。

## 交互禁用

### 禁用一项或整组

```rust
bsn! {
    Node
    InteractionDisabled
    Children [
        @WidgetryCheckBox Children [Text("选项 A") WidgetryText]--
        @WidgetryCheckBox Checked Children [Text("选项 B") WidgetryText]
    ]
}
```

祖先禁用传递给两项，但不会抹掉 B 的勾选。取消祖先限制后，原有状态仍然保留。单项也可以持有自己的 InteractionDisabled。

## 自定义内容与外观

### 使用标签和其他内容

默认 CheckBox 维护 Indicator，调用方通过 Children 提供标签、说明或其他 BSN 内容。主题和交互状态影响默认标记与受管内容配色。

### 只使用二态行为

```rust
app.add_plugins(WidgetryCheckBoxHeadlessPlugin);

let scene = bsn! {
    @WidgetryCheckBoxHeadless
    Node { width: px(96), height: px(36) }
    Checked
    Children [Text("自定义开关") Pickable::IGNORE]
};
```

Headless 不创建方框、勾选 SVG 或默认配色。外部 Style 自行读取 Checked、Hover 和禁用结果。三态也有独立的 `WidgetryTriStateCheckboxHeadless` 与对应行为 Plugin；它不依赖默认 Indicator。
