# ProgressBar

ProgressBar 展示应用已知的进度，不计算任务完成率，也不启动后台任务。它支持确定进度和不确定进度，二者可在运行时切换。

```rust
use bevy::prelude::*;
use bevy_widgetry::progress_bar::*;

app.add_plugins(WidgetryProgressBarPlugin);
```

## 确定进度

### 创建并更新百分比

```rust
bsn! {
    @WidgetryProgressBar {
        @state: {WidgetryProgressState::Determinate { value: 0.25 }}
    }
    Node { width: px(320) }
}
```

value 是 0–1 的比例，不是 0–100 的整数百分比。省略 state 时初始值为 0。

```rust
WidgetryProgressBar::set_state(
    &mut commands,
    root,
    WidgetryProgressState::Determinate { value: 0.75 },
);
```

允许进度增加、减少和重置。NaN、无穷和越界值被拒绝，不会自动 Clamp；非法更新保留已有状态。

### 根据实际工作量计算进度

例如完成 30 个文件、总数 120，提交 `30.0 / 120.0`。总数尚未知时使用不确定模式，不把除零或伪造百分比交给控件。

```rust
let state = if total == 0 {
    WidgetryProgressState::Indeterminate
} else {
    WidgetryProgressState::Determinate {
        value: completed as f32 / total as f32,
    }
};
WidgetryProgressBar::set_state(&mut commands, root, state);
```

这里的 completed 和 total 由业务保证一致，控件只验证最终比例。

## 不确定进度

### 在等待阶段显示持续活动

```rust
WidgetryProgressBar::set_state(
    &mut commands,
    root,
    WidgetryProgressState::Indeterminate,
);
```

模式切换后不保留一份隐藏的旧百分比。业务重新得知进度时显式提交新的 Determinate；控件不会猜测应该恢复到多少。

## 状态查询与同步修改

### 读取当前模式

```rust
fn update_progress(world: &mut World, root: Entity) -> Result {
    WidgetryProgressBar::set_state_in_world(
        world,
        root,
        WidgetryProgressState::Determinate { value: 1.0 },
    ).map_err(|error| error.into_bevy_error())?;

    let state = WidgetryProgressBar::state(world, root)
        .map_err(|error| error.into_bevy_error())?;
    info!(?state, "进度显示已更新");
    Ok(())
}
```

真实变化返回 true，合法同值返回 false，失败返回 ProgressBarFailure。Commands 入口在执行时校验，不能把排队本身当成已经更新。

## 外观

### 控制尺寸和主题

宽度由根 Node 控制；高度配置要求有限正数。Track、Fill、Border 使用当前主题，支持 `WidgetryProgressBarColorOverrides` 的局部覆盖以及 `get`、`set`、`clear` 同步/排队入口。

ProgressBar 是只读展示，没有 Hover、Pressed 或键盘输入语义。进度状态并不因为父级禁用就消失，完成后的文字、图标或业务成功消息由应用在外部组合。
