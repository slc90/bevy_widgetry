# Ui Scale

Ui Scale 为普通原生窗口统一应用一个应用级缩放比例，同时保留各窗口的逻辑尺寸基准。它与系统显示器 DPI 是不同配置，不要求应用把比例限制在固定档位。

Facade 需要启用 `ui_scale` Feature。

```rust
use bevy::prelude::*;
use bevy_widgetry::ui_scale::*;

app.insert_resource(WidgetryUiScale::new(1.25)
    .map_err(UiScaleFailure::into_bevy_error)?);
app.add_plugins(WidgetryUiScalePlugin);
```

## 应用缩放

### 运行期修改比例

```rust
fn enlarge(mut scale: ResMut<WidgetryUiScale>) -> Result {
    scale.set(1.5).map_err(UiScaleFailure::into_bevy_error)?;
    Ok(())
}
```

比例必须为有限正数。合法同值返回 `false`；非法值保留旧配置。`get()` 读取当前应用比例。

配置在窗口同步阶段应用，不是 Setter 返回时所有原生窗口已经完成 Resize。新加入的普通 Window 也会采用当前比例。

## 窗口尺寸

### 保留逻辑基准

应用缩放改变窗口物理尺寸，用户主动 Resize 则更新后续缩放所用的逻辑基准。原生尺寸约束依然有效，放宽约束后可以恢复之前未能达到的设计尺寸。

例如，设计尺寸为 800×600，比例从 1.0 改到 1.5 时按相同逻辑尺寸放大；用户在 1.5 下把窗口重新调整后，下一次缩放使用新的逻辑基准，而不是永远回到最初的 800×600。

不要同时用另一个系统逐帧强写同一窗口的物理尺寸来“修正”缩放，否则两套管理者会争夺输出。

## 排除窗口

### 保持某个窗口不受应用缩放管理

```rust
let fixed_window = world.spawn((
    Window { resolution: (480, 270).into(), ..default() },
    WindowScaleExcluded,
)).id();
```

与 Window 同批创建的排除标记也有效。管理器跳过其比例 Override、尺寸和策略。

### 暂停和重新接管

```rust
world.entity_mut(window).insert(WindowScaleExcluded);
world.entity_mut(window).remove::<WindowScaleExcluded>();
```

添加标记时保留当前窗口配置；移除后按当时逻辑尺寸重新接管，不恢复早已过时的实例基准。删除 Window 后重新建立也不会继承旧实例缓存。

## 与其他窗口功能协调

`WidgetryUiScaleSystems::Sync` 提供同步顺序点。需要读取最终窗口比例或尺寸的消费者可以排在其后，而不是从应用配置值直接推导一个尚未应用的几何快照。

Splash 使用自己的固定物理几何并带排除标记，不需要 Ui Scale 特判每种窗口业务。普通窗口的 Camera 与 Viewport 则消费本次实际窗口更新。
