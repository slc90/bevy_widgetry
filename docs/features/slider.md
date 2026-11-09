# Slider

Slider 提供一个有范围的 `f32` 数值，可以连续变化，也可以按步长变化。默认外观包含轨道、填充、滑块与焦点环；数值模型和输入策略可以分别复用。

```rust
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::slider::*;

app.add_plugins(WidgetrySliderPlugin);
```

## 数值范围

### 创建连续或步进滑块

```rust
bsn! {
    @WidgetrySlider {
        @min: 0.0,
        @max: 100.0,
        @value: {Some(25.0)}
    }
    Node { width: px(320) }
    on(value_changed)
}

fn value_changed(event: On<ValueChange<f32>>) {
    info!(root = ?event.source, value = event.value, "数值已改变");
}
```

省略配置时范围为 0–100，初值为 min。min、max、step 在构造后固定；范围必须有限且 min 小于 max。

```rust
bsn! {
    @WidgetrySlider {
        @min: 0.0,
        @max: 10.0,
        @step: {Some(3.0)},
        @value: {Some(4.0)}
    }
}
```

步进模式接受两个端点，以及从任一端点按步长可达的值。上例的 4 可从 10 反向到达，因此合法；4.5 不合法。程序化 Setter 不自动 Clamp 或吸附非法值。

### 查询和修改

```rust
fn set_volume(world: &mut World, root: Entity, value: f32)
    -> Result<bool, SliderFailure>
{
    WidgetrySliderHeadless::set_value_in_world(world, root, value)
}

// 排队设置，执行时校验。
WidgetrySliderHeadless::set_value(&mut commands, root, 50.0);
```

`WidgetrySliderHeadless::value(world, root)` 读取当前值；`state` 返回只读状态，包含范围与步长。真实变化返回 true 并通知，同值返回 false，非法值保留旧状态。Disabled 不阻止合法程序化设置。

## 拖拽与键盘

### 区分数值变化和手势结束

拖动中的数值变化携带 `is_final=false`，键盘和程序化修改携带 `is_final=true`。这个字段不代替拖拽生命周期事件。

```rust
fn drag_started(event: On<SliderDragStarted>) {
    info!(root = ?event.entity, initial = event.value, "开始拖动");
}

fn drag_ended(event: On<SliderDragEnded>) {
    info!(root = ?event.entity, value = event.value,
          outcome = ?event.outcome, "拖动结束");
}

let scene = bsn! {
    @WidgetrySlider
    on(value_changed)
    on(drag_started)
    on(drag_ended)
};
```

点击轨道会开始手势并更新值；按住 Thumb 则保留抓取偏移。即使没有移动，也会有完整 Start/End。取消保留当前值，不自动回滚；结束时不会为了通知结束而补发一次同值 ValueChange。

播放进度条需要在拖动前暂停时钟时，应在 Started 保存原播放状态，在 Ended 根据 outcome 决定后续动作，而不是等待最后一次 ValueChange。

### 理解方向和步长

水平方向向右增大，竖直方向向上增大。步进随移动方向从当前值递进：`0→3→6→9→10`，反向为 `10→7→4→1→0`。结果不依赖 Move 事件被拆成多少次。

取得焦点后，对应轴的方向键移动一步；连续模式每次为跨度的 1%。没有 Home/End 行为。失焦、有效禁用、取消或捕获丢失会结束手势，重新启用不会续接旧拖动。

## 尺寸与颜色

### 配置竖向滑块

```rust
bsn! {
    @WidgetrySlider {
        @orientation: WidgetrySliderOrientation::Vertical,
        @track_thickness: 8.0,
        @thumb_diameter: 24.0
    }
    Node { height: px(280) }
}
```

根主轴长度控制行程，Thumb 直径独立保持。默认主轴长度 200 像素，轨道厚度 6 像素、Thumb 直径 20 像素；厚度和直径必须为有限正数。

`WidgetrySliderColorOverrides` 按状态覆盖 track、fill、thumb、focus_ring；未覆盖字段跟随 Theme。根布局变化不直接修改数值。

## 独立数值与自定义输入

### 只使用数值模型

```rust
app.add_plugins(WidgetrySliderHeadlessPlugin);

let model = bsn! {
    @WidgetrySliderHeadless {
        @min: 0.0,
        @max: 1.0,
        @value: {Some(0.5)}
    }
};
```

这个实体不自动具有 Node、轨道、滑块或交互。它仍能使用相同查询与 Setter，适合外部数值读数或另一种控制器。

### 使用自己的交互策略

安装 `WidgetrySliderInteractionPlugin`，在同根组合 `WidgetrySliderInteraction::new(strategy)`。策略从只读 World 读取布局和实例配置，把指针位置映射为连续数值；库负责 Clamp、步进、捕获、提交和取消。

每次拖动的轨迹保存在独立 `SliderDragStrategy` 中。策略可以返回 Position、Wait 或 Cancel，不直接改写数值。默认公开线性绑定 `WidgetrySliderLinear { track, thumb, orientation }` 也可以指向应用自有节点。

当前 Slider 是单值控件，没有 Range Slider、刻度或内置数值标签；这些内容可在外层组合。
