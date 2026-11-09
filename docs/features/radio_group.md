# RadioGroup

RadioGroup 提供严格单选的选项集合，支持运行时增删选项、键盘切换，以及默认与自定义选项混合使用。

```rust
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::radio_group::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryRadioGroupPlugin);
```

## 单选集合

### 创建组和选项

```rust
bsn! {
    @WidgetryRadioGroup
    Node { flex_direction: FlexDirection::Row, column_gap: px(16) }
    on(selection_changed)
    Children [
        @WidgetryRadioOption Checked
        Children [Text("日视图") WidgetryText]--
        @WidgetryRadioOption
        Children [Text("周视图") WidgetryText]--
        @WidgetryRadioOption
        Children [Text("月视图") WidgetryText]
    ]
}

fn selection_changed(event: On<ValueChange<usize>>) {
    info!(group = ?event.source, index = event.value, "用户切换选项");
}
```

每个 Option 必须是 Group 的直接 Child，Option 内部可有自己的内容层级。合法集合非空，并保持恰好一项 Checked。不要在 Group 与 Option 之间额外包一层布局 Node；需要布局时直接修改 Group 的 Node。

### 程序化选择

```rust
WidgetryRadioGroupHeadless::set_selected(&mut commands, group, 1);
```

索引按命令执行时的直接 Children 顺序解释。程序化选择不发送用户选择通知；同值操作静默。索引越界、空组或结构非法会返回或传播错误，不改变已有选择。

## 动态选项

### 增加和删除

新增 Option 的 Scene 组合 `ChildOf(group)`，删除使用正常 ECS Despawn：

```rust
let new_option = bsn! {
    @WidgetryRadioOption
    ChildOf(group)
    Children [Text("季度视图") WidgetryText]
};

commands.entity(option_to_remove).despawn();
```

将 `new_option` 交给当前应用的 Scene 构造入口即可。新增项不会凭自身 Checked 抢走已经合法的选择。当前不支持任意重排；不要把程序化索引当成持久业务 ID。

### 理解删除后的恢复

删除未选中项不改变选中实体。删除已选中项后，先按原有合法顺序寻找仍存在的后继，再寻找前驱；旧选项全部消失时，选择新集合首项。结构恢复不发送用户选择事件，也不会因为替补项 Disabled 而放弃恢复。

## 焦点与禁用

### 禁用一项或整组

```rust
bsn! {
    @WidgetryRadioGroup
    Children [
        @WidgetryRadioOption Checked Children [Text("可用") WidgetryText]--
        @WidgetryRadioOption InteractionDisabled
        Children [Text("暂不可选") WidgetryText]
    ]
}
```

Group 的 Disabled 继承给所有选项，单项禁用独立保留。禁用阻止用户选择，合法程序化设置仍然可用。

指针选择使 Group 取得焦点，有焦点时可以用方向键切换。插件不自动安装跨控件 Tab 导航；默认 `TabIndex(-1)` 主要用于指针聚焦。

## 自定义外观

### 使用行为组和行为选项

```rust
app.add_plugins(WidgetryRadioGroupHeadlessPlugin);

let cards = bsn! {
    @WidgetryRadioGroupHeadless
    Node { column_gap: px(12) }
    Children [
        @WidgetryRadioOptionHeadless Checked
        Node { padding: UiRect::all(px(12)) }
        Children [Text("卡片 A") Pickable::IGNORE]--
        @WidgetryRadioOptionHeadless
        Node { padding: UiRect::all(px(12)) }
        Children [Text("卡片 B") Pickable::IGNORE]
    ]
};
```

默认和自定义 Option 可以混合。默认 Style 只管理默认身份，不改写自定义卡片的颜色；默认 Option 位于纯 Headless Group 下时使用自身覆盖与 Theme。
