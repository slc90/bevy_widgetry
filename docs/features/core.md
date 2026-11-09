# Core 共享 UI 能力

Core 提供文本与图标、有效禁用、Scene 错误适配、输入协调和离屏渲染等公共能力。普通应用优先使用 Facade 中的公开入口；编写库内扩展时可以直接依赖 `bevy_widgetry_core`。

## UI 基础装配

### 安装共享 UI 机制

```rust
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiPlugin;

app.add_plugins(WidgetryUiPlugin);
```

插件协调 Disabled、Pointer、前景色、Theme 与 Cursor 等基础能力。它不替代 Bevy 的 Asset、UI、窗口和渲染插件，也不自动为应用创建业务界面。

普通控件插件会确保自己的共享依赖；宿主显式装配时避免重复添加相同插件。

## 文本与默认字体

### 使用跟随控件前景色的文本

```rust
use bevy_widgetry::text::WidgetryText;

bsn! {
    Text("保存")
    WidgetryText
}
```

Widgetry 标记文本参与共享前景色更新。未标记的普通 Text 保留自己的颜色定义，不会因为位于某个控件内部就被无条件改色。

### 设置应用默认字体

```rust
use bevy::text::FontSource;
use bevy_widgetry::style::WidgetryAppExt;

app.set_default_font(FontSource::monospace());
```

也可以使用已加载的字体 Handle 构造 `FontSource::Handle`。默认字体应用于尚未显式指定字体的新 TextFont，并补充 Emoji 字体后备；它不是任意时刻强行覆盖所有既有文本的主题开关。

只需要内建默认字体时可直接安装 `bevy_widgetry_core::WidgetryFontPlugin`。

## SVG 图标

### 创建图标

```rust
use bevy_widgetry::icon::{WidgetryIcon, WidgetryIconPlugin};

app.add_plugins(WidgetryIconPlugin);

let icon = bsn! {
    @WidgetryIcon {
        @path: {"icons/save.svg".into()},
        @max_size: {Some(UVec2::splat(24))}
    }
};
```

路径属于应用的 Asset 来源。图标按 SVG 内容栅格化，可参与控件前景色与自身颜色配置。`max_size` 是可选的最大栅格规格，不把实际 UI 布局改成另一套坐标体系。

资源等待与加载失败不同。等待时允许保留尚未显示或已经存在的图像，失败通过诊断路径反馈；颜色和资产变化由图标能力负责更新，不需要调用方直接改内部 ImageNode。

## 有效 Disabled

### 提交和解除显式禁用

```rust
use bevy::ui::InteractionDisabled;

commands.entity(control).insert(InteractionDisabled);
// 解除自己的显式请求：
commands.entity(control).remove::<InteractionDisabled>();
```

父级禁用会沿 UI 层级生效。解除当前实体的显式请求，不会撤销父级或内部行为仍持有的限制。

### 查询最终结果

```rust
use bevy_widgetry::disabled::WidgetryEffectiveDisabled;

fn inspect_disabled(states: Query<(Entity, &WidgetryEffectiveDisabled)>) {
    for (entity, state) in &states {
        if state.is_disabled() {
            debug!(?entity, "当前有效禁用");
        }
    }
}
```

不要用是否存在本地业务标记来推断完整禁用结果。Commands 尚未执行时，新请求还没有提交。

### 为独立行为登记内部理由

库内扩展使用理由类型隔离不同限制：

```rust
struct WaitingForReply;

bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<WaitingForReply>(
    world, control, true,
).map_err(|failure| failure.into_bevy_error())?;

bevy_widgetry_core::disabled::set_intrinsic_disabled_for::<WaitingForReply>(
    world, control, false,
).map_err(|failure| failure.into_bevy_error())?;
```

同一种理由在同一实体上是有或无，不是重复调用计数。每个行为负责撤销自己的理由。

## Scene 构造与失败

### 同步创建 Scene

```rust
use bevy_widgetry::scene;

let root = scene::spawn_scene(world, bsn! {
    Node { width: px(200), height: px(40) }
    Children [Text("内容") WidgetryText]
}).map_err(|failure| failure.into_bevy_error())?;
```

同步入口返回根 Entity；构造失败通过 SceneFailure 保留真实原因，并清理这次新建的根。

### 在 Commands 中创建

```rust
use bevy_widgetry::scene::WidgetrySceneCommandsExt;

let root = commands.spawn_scene_with_error_handler(bsn! {
    Node
    Children [Text("延迟构造") WidgetryText]
}).id();
```

此时返回预约实体，实际构造在 Commands 执行时发生。错误交给宿主 Handler；预约根在执行前已经正常销毁时视为取消。

`apply_scene` 与对应 EntityCommands 扩展可用于现有实体。它们不是业务事务回滚系统，已经提交的独立业务状态不因视觉构造失败而自动恢复旧值。

## 输入目标与调度

组合控件的逻辑根不一定是真正的文本编辑或键盘输入实体。Core 的输入目标元数据让窗口聚焦等能力定位到实际目标，而控件继续拥有自己的业务和编辑状态。

共享 UI 阶段包括构造、物化、Disabled、样式归属、颜色和前景传播。扩展系统应按自己消费的数据选择阶段，而不是额外建立独立 UI 循环。完整关系见 [输入机制](../design/mechanisms/input-interaction.md) 和 [控件组合](../design/mechanisms/ui-composition.md)。

## 离屏目标

自定义 Renderer 可使用 `WidgetryOffscreenPlugin`、`OffscreenTarget` 和 `OffscreenViewport`。公共层提供目标、层号、Viewport 关联和回收，消费者负责自己的相机配置、绘制 Scene 与 Mesh/Material。

分配后尚未完成绑定而构造失败时，由调用方释放目标；成功绑定后由拥有者组件生命周期管理目标。隐藏或零尺寸是暂停渲染，不是销毁业务模型。具体资源关系见 [离屏渲染机制](../design/mechanisms/offscreen-rendering.md)。
