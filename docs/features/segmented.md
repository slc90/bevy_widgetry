# Segmented

Segmented 提供横向或竖向的严格单选分段控件，适合切换时间范围、视图模式和少量互斥选项。默认外观使用选中浮块，也支持纯行为组合。

```rust
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::segmented::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetrySegmentedPlugin);
```

## 分段选择

### 创建选项集合

```rust
bsn! {
    @WidgetrySegmented
    on(changed)
    Children [
        @WidgetrySegmentedOption Checked Children [Text("日") WidgetryText]--
        @WidgetrySegmentedOption Children [Text("周") WidgetryText]--
        @WidgetrySegmentedOption Children [Text("月") WidgetryText]
    ]
}

fn changed(event: On<ValueChange<usize>>) {
    info!(root = ?event.source, index = event.value, "切换分段");
}
```

Group 的直接 Children 必须全部为 Option，至少一项。没有初始 Checked 时选择首项；多个初始 Checked 属于冲突，不会静默覆盖。

### 程序化切换

```rust
WidgetrySegmentedHeadless::set_selected(&mut commands, group, 2);
```

索引取命令执行时的成员顺序。程序设置与真实用户输入的通知语义分别保留；初始化、合法同值和自动结构修复不伪造用户事件。

## 方向与布局

### 使用水平等宽分段

```rust
bsn! {
    @WidgetrySegmented { @equal_width: true }
    Node { width: px(360) }
    Children [
        @WidgetrySegmentedOption Children [Text("最近一天") WidgetryText]--
        @WidgetrySegmentedOption Children [Text("最近一周") WidgetryText]--
        @WidgetrySegmentedOption Children [Text("最近一月") WidgetryText]
    ]
}
```

`equal_width` 是默认 Style 的布局能力，不改变单选状态。未开启时按内容自然布局。

### 改为竖向

```rust
bsn! {
    @WidgetrySegmented { @orientation: SegmentedOrientation::Vertical }
    Children [
        @WidgetrySegmentedOption Children [Text("列表") WidgetryText]--
        @WidgetrySegmentedOption Children [Text("详细信息") WidgetryText]
    ]
}
```

方向同时决定本控件的方向键语义。焦点内的导航不循环，不等同于跨控件 Tab 导航。

## 禁用与动态成员

### 保留禁用项的选择

```rust
bsn! {
    @WidgetrySegmented
    Children [
        @WidgetrySegmentedOption Children [Text("本地") WidgetryText]--
        @WidgetrySegmentedOption Checked InteractionDisabled
        Children [Text("远程") WidgetryText]
    ]
}
```

禁用不会清除已选项。Group 禁用继承给成员；解除 Group 禁用后，单项自身的禁用仍保留。

成员增删使用 ECS 父子关系。删除选中项时按此前合法顺序恢复，集合为空时报告结构错误；后续恢复合法成员可以重新建立选择。

## 自定义外观

### 使用 Headless 组合自己的选项

```rust
app.add_plugins(WidgetrySegmentedHeadlessPlugin);

let custom = bsn! {
    @WidgetrySegmentedHeadless {
        @orientation: SegmentedOrientation::Horizontal
    }
    Node { column_gap: px(16) }
    Children [
        @WidgetrySegmentedOptionHeadless Checked
        Children [Text("概览") Pickable::IGNORE]--
        @WidgetrySegmentedOptionHeadless
        Children [Text("详情") Pickable::IGNORE]
    ]
};
```

Headless 保存方向、输入与选择规则，不创建浮块和默认颜色。自定义 Option 可以与默认 Option 同组；默认外观只更新自己拥有的部件。
