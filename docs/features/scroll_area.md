# ScrollArea

ScrollArea 为超过可见范围的内容提供滚动视口、滚动条和键盘导航。滚动方向与滚动条是否显示分别配置；隐藏滚动条不等于关闭滚动。

```rust
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy_widgetry::scroll_area::*;
use bevy_widgetry::text::WidgetryText;

app.add_plugins(WidgetryScrollAreaPlugin);
```

## 内容滚动

### 创建纵向滚动区域

给根节点设置有限的可见高度，把内容放进 `@children`。`@content` 修饰内容容器，而不是外部视口。

```rust
bsn! {
    @WidgetryScrollArea {
        @content: {Some(Box::new(bsn! {
            Node { flex_direction: FlexDirection::Column, row_gap: px(8) }
        }) as Box<dyn Scene>)},
        @children: {Some(Box::new(bsn_list! {
            Text("任务说明") WidgetryText--
            Node { height: px(600), flex_shrink: 0.0 }
            Children [Text("这部分内容超过视口高度") WidgetryText]
        }) as Box<dyn SceneList>)},
    }
    Node { width: px(320), height: px(220) }
}
```

默认使用纵向滚动。纵向模式将内容宽度约束在视口内，文字可以按视口宽度换行；横向和双轴模式允许内容保留固有宽度。内容没有溢出时不会强行制造滚动范围。

### 选择滚动方向

```rust
bsn! {
    @WidgetryScrollArea {
        @axis: ScrollAxis::Both,
        @children: {Some(Box::new(bsn_list! {
            Node { width: px(900), height: px(600), flex_shrink: 0.0 }
            Children [Text("较大的二维内容") WidgetryText]
        }) as Box<dyn SceneList>)},
    }
    Node { width: px(400), height: px(240) }
}
```

`Horizontal`、`Vertical`、`Both` 决定真正允许滚动的轴。内容裁剪、偏移和滚动条几何跟随实际布局更新。根节点需要明确可用空间，否则内容可能直接撑开容器，无法形成预期的滚动窗口。

## 滚动条

### 独立配置两条滚动条

```rust
bsn! {
    @WidgetryScrollArea {
        @axis: ScrollAxis::Both,
        @scrollbar_visibility: ScrollbarVisibility {
            horizontal: ScrollbarPolicy::Hidden,
            vertical: ScrollbarPolicy::Always,
        },
        @scrollbar_thickness: 10.0,
        @children: {Some(Box::new(bsn_list! {
            Node { width: px(800), height: px(600), flex_shrink: 0.0 }
        }) as Box<dyn SceneList>)},
    }
    Node { width: px(360), height: px(240) }
}
```

| 策略 | 效果 |
| --- | --- |
| `Auto` | 有对应轴溢出时显示 |
| `Always` | 保留对应滚动条 |
| `Hidden` | 不显示滚动条，但不关闭该轴的滚动 |

滚动条占用空间可能改变另一轴的溢出情况，最终显示结果按布局求解。Thumb 的长度和位置由可见范围与内容范围决定，不应直接修改其内部布局来实现滚动。

## 程序化滚动

### 修改视口偏移

真正保存偏移的是带有 `WidgetryScrollAreaViewport` 的视口实体，而不是默认控件根。下面的系统把已经定位好的视口向下滚动 96 像素：

```rust
fn scroll_down(
    viewport: Entity,
    positions: &mut Query<&mut ScrollPosition, With<WidgetryScrollAreaViewport>>,
) -> Result {
    positions.get_mut(viewport)?.0.y += 96.0;
    Ok(())
}
```

多实例应用应保存或定位目标视口，不要把所有 `WidgetryScrollAreaViewport` 一起移动。布局阶段会根据合法范围修正偏移；内容缩小、视口变大时也会重新协调。

### 将内容移入可见范围

向需要显示的后代实体发送请求：

```rust
commands.trigger(WidgetryScrollIntoView { entity: target });
```

请求只处理最近的 ScrollArea。目标已经完整可见时不改变位置。嵌套滚动区域需要分别考虑各层可见性，单个请求不会自动展开整条外层滚动链。

## 用户输入

### 配置键盘滚动

默认启用键盘滚动，纵向支持 PageUp、PageDown、Home 和 End。关闭键盘滚动不影响滚轮、Thumb 拖动或程序化偏移。

```rust
bsn! {
    @WidgetryScrollArea {
        @keyboard_scroll: false,
        @children: {Some(Box::new(bsn_list! {
            Text("只保留滚轮与程序控制") WidgetryText
        }) as Box<dyn SceneList>)},
    }
    Node { width: px(320), height: px(180) }
}
```

禁用控件会停止用户滚轮及滚动条交互，但保留 `ScrollPosition`、内容布局和程序化滚动。重新启用不会把视口重置到开头。

## 外观与颜色

使用 `WidgetryScrollAreaColorOverrides` 的 `get`、`set_in_world` / `set`、`clear_in_world` / `clear` 管理覆盖。Thumb 的视觉优先级为 Disabled、Dragged、Hovered、Normal。

```rust
WidgetryScrollAreaColorOverrides::clear(&mut commands, scroll_root);
```

这里传入的是 ScrollArea 根实体。组合控件托管的滚动区域使用其所属控件提供的颜色，不应绕过所属控件直接覆盖内部部件。
