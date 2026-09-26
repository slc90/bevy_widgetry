# Icon 构造 API BSN 化方案

## 目标

Icon 统一通过 BSN 的 `@Icon` 创建，配置作为原生 Scene props 传入。

`Icon` 同时承担 SceneComponent 入口与运行期 ECS Component 的职责：

- Scene 构造阶段接收一次性 props；
- 展开后保存 SVG handle、尺寸和颜色状态；
- 供 Icon systems 查询和更新；
- 提供运行期 `set_svg / set_color / clear_color`。

删除 `IconConfig`、`icon()` 和 `icon_with()`，不保留公开的 `Icon::new / with_size / with_color`。

SVG 异步加载、栅格化、缓存、materialize 和颜色传播机制保持不变。

## 公共 BSN API

默认使用 SVG 原始尺寸和继承前景色：

```rust
bsn! {
    @Icon {
        @path: "icons/save.svg",
    }
}
```

显式指定尺寸和颜色：

```rust
bsn! {
    @Icon {
        @path: "icons/save.svg",
        @max_size: { Some(UVec2::new(16, 16)) },
        @color: { Some(Color::WHITE) },
    }
}
```

`@` 字段表示 BSN props；大括号内的 Rust 表达式用于传入 Option 等值。
路径也可以来自拥有所有权的字符串或语义资源标识：

```rust
bsn! {
    @Icon {
        @path: {BuiltinIcon::WindowClose.path()},
        @max_size: { Some(UVec2::new(16, 16)) },
        @color: { Some(Color::WHITE) },
    }
}
```

对应 props 类型：

```rust
#[derive(Clone, Debug, Default)]
pub struct IconProps {
    pub path: AssetPath<'static>,
    pub max_size: Option<UVec2>,
    pub color: Option<Color>,
}
```

- 调用方必须提供有效的 `path`；Default 用于 BSN props 的初始化，不代表存在默认图标资源。
- `max_size: None` 使用 SVG 原始尺寸；Some 表示等比缩放上限，任一维为零时不生成图像。
- `color: None` 使用继承前景色；没有继承前景色时使用白色。
- `color: Some(...)` 优先于继承前景色。
- 创建前注册 AssetPlugin、ScenePlugin 和 IconPlugin，通常由应用及控件插件完成装配。
- 消费者无需取得 AssetServer，也不需要显式构造 IconProps。

## Props 与运行期状态

遵守 `rules/architecture.md` 的 Scene props 规则：

1. props 只用于一次性初始化，允许初始化同义 Component 字段。
2. Scene 展开完成后，props 的语义生命周期结束。
3. Icon Component 是 SVG handle、尺寸和颜色的唯一运行期状态来源。
4. 不保存同义的 props 状态副本，也不在 props 与 Component 之间持续同步。

因此，使用 `@max_size` 和 `@color` 初始化 Icon 字段是允许的；运行期修改直接作用于 Icon，不会被初始 props 覆盖。

## Scene 实现

Icon 派生 SceneComponent 和 FromTemplate，并关联 IconProps：

```rust
#[derive(SceneComponent, FromTemplate)]
#[scene(IconProps)]
#[require(Node, IconRasterState)]
pub struct Icon {
    svg: Handle<svg::SvgAsset>,
    max_size: Option<UVec2>,
    color: Option<Color>,
}
```

内部 scene 函数将 props 写入组件模板：

```rust
impl Icon {
    fn scene(props: IconProps) -> impl Scene {
        bsn! {
            Icon {
                svg: {props.path},
                max_size: {props.max_size},
                color: {props.color},
            }
        }
    }
}
```

FromTemplate 将 SVG handle 字段转为 Bevy 原生句柄模板，在 Scene 展开时通过上下文中的 AssetServer 请求资源。SceneComponent 自动附加 Icon 身份，不需要额外的场景身份组件。

职责仅限：

```text
IconProps
    ↓
Icon 组件模板
    ↓ Scene 展开，句柄模板取得 AssetServer
Icon 运行期组件
```

以下职责继续由现有 Icon systems 负责：

- SVG 异步等待；
- SVG rasterization；
- ImageNode 子实体创建；
- 图像缓存；
- pending update；
- foreground color 同步。

## Window 与 Gallery 调用迁移

Window 的 `system_icon()` 保留窗口自己的固定策略：16 × 16、白色、Pickable::IGNORE。

```rust
fn system_icon(icon: BuiltinIcon) -> impl Scene {
    bsn! {
        @Icon {
            @path: {icon.path()},
            @max_size: { Some(UVec2::new(16, 16)) },
            @color: { Some(Color::WHITE) },
        }
        template(|_| Ok(Pickable::IGNORE))
    }
}
```

Window 不在此函数中访问 AssetServer。

Gallery Logo 同样迁移为 `@Icon`，使用 GalleryIcon::Logo 的语义路径与 16 × 16 尺寸，省略 `@color` 以继续继承主题前景色。

## 测试中的构造迁移

Window 还原图标测试统一使用 BSN：

```rust
app.world_mut().commands().spawn_scene(bsn! {
    @Icon {
        @path: {BuiltinIcon::WindowRestore.path()},
        @max_size: { Some(UVec2::new(16, 16)) },
        @color: { Some(Color::WHITE) },
    }
});
```

内部栅格化故障测试也通过 Scene 创建 Icon；可在模块内部覆盖私有句柄模板，以确定性地控制测试资源的就绪时机。不得为测试保留公开的普通 ECS 构造器。

## 运行期 API

以下方法继续公开：

```rust
Icon::set_svg(...)
Icon::set_color(...)
Icon::clear_color(...)
```

Window 最大化与还原状态同步仍查询 `Query<&mut Icon>`，通过 `set_svg` 请求对应资源。新 SVG 加载完成前保留当前图像。这些操作属于已有实体的运行期状态修改。

## Facade API

```rust
pub mod icon {
    pub use bevy_widgetry_core::icon::{Icon, IconPlugin, IconProps};
}
```

消费者从 facade 导入 Icon 即可使用 `@Icon`，也可以查询 Icon 并调用运行期方法。

## 验证

Core 测试验证：

1. `spawn_scene(bsn! { @Icon { @path: ... } })` 生成 Icon 及其必需的 Node。
2. 省略可选 props 时，尺寸和颜色维持默认语义。
3. 显式 props 正确初始化尺寸与颜色，并接受拥有所有权的路径。
4. 展开后运行期 SVG 和颜色修改保持有效，不被初始 props 覆盖。

`crates/bevy_widgetry/tests/public_api.rs` 从消费者视角验证默认与显式 props 构造，确保 facade 可直接用于 BSN，且消费者不需要取得 AssetServer。

Window 的 `embedded_control_icons_materialize` 继续验证内嵌图标实际加载与图像生成。保留现有运行期回归测试，不为 props 重写栅格化、缓存或颜色传播机制。

Gallery 通过编译和实际运行检查 Logo、系统图标及最大化后的还原图标显示。

## 最终构造模型

```text
BSN @Icon + 一次性 IconProps
               │
               ▼
         Icon Component
               │
        runtime systems
               │
               ▼
         ImageNode child
```

BSN 负责一次性初始化，Component 保存运行期状态，ECS systems 负责异步图像生成与后续同步。
