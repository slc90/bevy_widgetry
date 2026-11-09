# Splitter

Splitter 将区域分为 Pane A、拖动 Handle 和 Pane B。支持横向或纵向分隔、面板原生尺寸约束和程序化比例更新。

```rust
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::splitter::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetrySplitterPlugin);
```

## 双面板布局

### 创建左右分栏

```rust
bsn! {
    @WidgetrySplitter {
        @split_ratio: 0.3,
        @pane_a: {Some(Box::new(bsn! {
            Node { min_width: px(160), padding: UiRect::all(px(12)) }
            Children [Text("导航") WidgetryText]
        }) as Box<dyn Scene>)},
        @pane_b: {Some(Box::new(bsn! {
            Node { min_width: px(240), padding: UiRect::all(px(12)) }
            Children [Text("内容") WidgetryText]
        }) as Box<dyn Scene>)}
    }
    Node { width: px(800), height: px(400) }
    on(ratio_changed)
}

fn ratio_changed(event: On<ValueChange<f32>>) {
    info!(root = ?event.source, ratio = event.value, "分隔比例变化");
}
```

默认 Horizontal 为 A 左 B 右。比例表示扣除 Handle 厚度后 A 占主轴空间的份额，默认 0.5。省略 Pane Scene 表示空面板。

### 创建上下分栏

```rust
bsn! {
    @WidgetrySplitter {
        @orientation: WidgetrySplitterOrientation::Vertical,
        @handle_thickness: 12.0,
        @pane_a: {Some(Box::new(bsn! { Children [Text("上部") WidgetryText] }) as Box<dyn Scene>)},
        @pane_b: {Some(Box::new(bsn! { Children [Text("下部") WidgetryText] }) as Box<dyn Scene>)}
    }
    Node { height: px(480) }
}
```

Pane 内可以嵌套 Splitter。滚动、裁剪和交叉轴布局由调用方在各 Pane 中组合。

## 比例控制

### 查询和程序化更新

```rust
fn split_evenly(world: &mut World, root: Entity) -> Result {
    WidgetrySplitter::set_split_ratio_in_world(world, root, 0.5)
        .map_err(|error| error.into_bevy_error())?;
    let ratio = WidgetrySplitter::split_ratio(world, root)
        .map_err(|error| error.into_bevy_error())?;
    info!(ratio, "当前目标比例");
    Ok(())
}
```

`set_split_ratio` 为排队版本。比例必须有限且属于 0–1；非法请求保留旧状态。同步返回值区分真正变化与合法同值。

### 理解窗口缩小时的表现

Pane 的 min/max 可能使目标比例暂时不可实现。被动 Resize 或约束变化影响实际几何，但保留目标比例；空间恢复后再次尝试该目标，不把每一次临时挤压永久写成新偏好。

## 面板约束

### 设置最小和最大尺寸

```rust
bsn! {
    @WidgetrySplitter {
        @pane_a: {Some(Box::new(bsn! {
            Node { min_width: px(180), max_width: px(320) }
        }) as Box<dyn Scene>)},
        @pane_b: {Some(Box::new(bsn! {
            Node { min_width: px(240) }
        }) as Box<dyn Scene>)}
    }
}
```

约束使用 Pane 原生 Node 的主轴字段。ContentBox 的 padding/border 计入外部占用，BorderBox 不重复计算。百分比约束随根内容尺寸重新求解，不固化为首帧像素值。

父级容不下两侧最小尺寸与 Handle 时，保留溢出，应用可以在外层加 ScrollArea 或裁剪。控件不会偷偷缩小最小值或隐藏面板。

## 拖拽与外观

### 固定比例但保留面板内容交互

```rust
bsn! {
    @WidgetrySplitter {
        @resizable: false,
        @split_ratio: 0.4,
        @pane_a: {Some(Box::new(bsn! { Children [Text("固定侧栏") WidgetryText] }) as Box<dyn Scene>)},
        @pane_b: {Some(Box::new(bsn! { Children [Text("主区域") WidgetryText] }) as Box<dyn Scene>)}
    }
}
```

resizable 关闭 Handle 拖动，不等同于禁用整个子树。根 Disabled 则按共享禁用机制影响用户交互。

Handle 厚度与可见分隔线厚度分别配置。Splitter 托管内部三节点的主轴分配，不直接修改它们的受管尺寸字段；Pane 内容样式和交叉轴几何仍由应用控制。
