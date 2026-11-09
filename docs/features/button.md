# Button

Button 提供默认按钮、链接按钮，以及可以自由组合外观的按钮行为。普通按钮的内容由调用方提供；链接按钮显示目标 URL，并在激活时请求系统浏览器打开。

本页示例使用以下导入，应用安装 `WidgetryButtonPlugin`。`root` 表示按钮根实体，不是内部文字或旋转指示器。

```rust
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;
use bevy_widgetry::button::*;
use bevy_widgetry::text::WidgetryText;

// 在已有 Bevy App 中装配。
app.add_plugins(WidgetryButtonPlugin);
```

## 普通按钮

### 创建并响应按钮

通过 `@children` 提供内容，通过根 `Node` 设置外部尺寸，通过 `on(...)` 响应激活。

```rust
fn save_button() -> impl Scene {
    bsn! {
        @WidgetryButton {
            @children: {Some(Box::new(bsn_list! {
                Text("保存") WidgetryText
            }) as Box<dyn SceneList>)}
        }
        Node { width: px(160), height: px(40) }
        on(save)
    }
}

fn save(event: On<Activate>) {
    info!(button = ?event.entity, "收到保存请求");
}
```

鼠标主键完成有效点击后激活；取得焦点后，Enter 或 Space 的有效按下也会激活。副键、取消、拖动以及键盘重复按下不会产生同样的按钮激活。

### 排列按钮内容

`@content` 修改内部内容容器，适合设置内容间距和排列方向。根节点的宽高、Padding 与内部排列分别配置。

```rust
bsn! {
    @WidgetryButton {
        @content: {Some(Box::new(bsn! {
            Node {
                column_gap: px(8),
                justify_content: JustifyContent::Center
            }
        }) as Box<dyn Scene>)},
        @children: {Some(Box::new(bsn_list! {
            Text("保存") WidgetryText--
            Text("Ctrl+S") WidgetryText
        }) as Box<dyn SceneList>)}
    }
    Node { min_width: px(180), height: px(40) }
    on(save)
}
```

业务内容放在 `@children` 中，不绕过内容容器直接追加到根 `Children`。状态切换不会重新构造这些业务内容。

## 交互禁用

### 设置和解除本地禁用

```rust
commands.entity(root).insert(InteractionDisabled);

// 只撤销本地显式请求；祖先或 Loading 仍可使按钮保持禁用。
commands.entity(root).remove::<InteractionDisabled>();
```

禁用会阻止正常用户输入产生新的激活。按钮还会继承 UI 祖先的禁用状态。默认 `TabIndex(-1)` 保留点击聚焦，跨控件 Tab 导航由应用单独配置。

直接由程序触发的 `Activate` 不经过 Headless 的用户输入检查。Disabled 不是业务权限检查或任务并发锁。

## Loading

### 开始和结束等待

普通 `WidgetryButton` 自带初始为 `false` 的 Loading，不需要初始化 Prop。业务在请求发起时开启，在成功、失败或取消后关闭。

```rust
fn begin_save(event: On<Activate>, mut commands: Commands) {
    WidgetryButtonLoading::set(&mut commands, event.entity, true);
    // 将保存请求交给业务系统，并保留 event.entity 供完成时使用。
}

fn finish_save(commands: &mut Commands, root: Entity) {
    WidgetryButtonLoading::set(commands, root, false);
}
```

`set` 在 Commands 实际执行时提交。需要同步结果时使用 World API：

```rust
fn set_busy(world: &mut World, root: Entity, busy: bool)
    -> Result<bool, ButtonFailure>
{
    WidgetryButtonLoading::set_in_world(world, root, busy)
}
```

`Ok(true)` 表示状态改变，`Ok(false)` 表示合法同值请求，失败返回 `ButtonFailure`。排队到生效之间不提供任务并发保护。

### 查询和监听变化

```rust
fn is_busy(world: &World, root: Entity) -> Result<bool, ButtonFailure> {
    Ok(WidgetryButtonLoading::get(world, root)?.is_active())
}

fn loading_changed(event: On<WidgetryButtonLoadingChanged>) {
    info!(button = ?event.entity, busy = event.loading, "等待状态已提交");
}
```

把 `on(loading_changed)` 挂到按钮根即可接收通知。通知只在实际改变后发出，表示状态已经提交，不表示画面同步已完成。

### 理解等待期间的反馈

默认外观隐藏内容并显示居中旋转指示器，保留布局占位和内容实体，声明等待光标并阻止新的用户激活。结束 Loading 只撤销 Loading 自己的禁用理由；业务或祖先仍然禁用时，按钮不会被错误启用。

## 外观与颜色

### 覆盖状态颜色

默认外观按 `Loading → Disabled → Pressed → Hovered → Normal` 选择视觉状态。Loading 使用 Normal 配色，旋转指示器表达等待。

`WidgetryButtonColorOverrides` 的四个状态分别支持背景、边框和前景色；未覆盖的字段继续跟随 Theme。

```rust
let overrides = WidgetryButtonColorOverrides {
    normal: WidgetryButtonStateColorOverrides {
        background: Some(Color::srgb(0.20, 0.40, 0.80)),
        foreground: Some(Color::WHITE),
        ..default()
    },
    hovered: WidgetryButtonStateColorOverrides {
        background: Some(Color::srgb(0.26, 0.48, 0.92)),
        ..default()
    },
    ..default()
};
WidgetryButtonColorOverrides::set(&mut commands, root, overrides);
```

构造时也可通过 `@colors` 提供同一种覆盖值。运行时 Setter 提交的是整份覆盖配置；需要只改一个字段时，先 `get(world, root)?.clone()`，再修改并提交。

### 恢复主题配色

```rust
WidgetryButtonColorOverrides::clear(&mut commands, root);
```

`get`、`set_in_world`、`clear_in_world` 提供同步入口。非法颜色和不合法目标不会替换原有配置。

## LinkButton

### 创建和更新链接

```rust
bsn! {
    @WidgetryLinkButton {
        @url: {"https://bevy.org/learn/".to_owned()}
    }
    Node { width: px(280) }
}
```

LinkButton 显示完整 URL 原文，长地址在给定宽度内换行。Hovered、Pressed 有下划线；Normal 和 Disabled 无下划线。它没有普通按钮的背景、边框和 Loading。

```rust
WidgetryOpenUrl::set_url(
    &mut commands,
    link_root,
    "https://bevy.org/",
);
```

修改 URL 不触发浏览器、不产生 `Activate`。默认文字在后续展示更新中同步。`WidgetryOpenUrl::url(world, link_root)` 读取当前已提交地址；`set_url_in_world` 同步返回是否实际改变。

### 配置链接颜色

```rust
bsn! {
    @WidgetryLinkButton {
        @url: {"https://bevy.org/".to_owned()},
        @colors: {WidgetryLinkButtonColorOverrides {
            normal: Some(Color::srgb(0.25, 0.55, 0.95)),
            ..default()
        }}
    }
}
```

链接覆盖类型支持四态文字颜色，也有 `get`、`set`、`clear` 及同步入口。

### 处理链接边界

地址必须是包含主机的完整 HTTP(S) URL；相对路径、空白、控制字符、反斜杠及其他协议会被拒绝，原 URL 保持不变。浏览器请求执行时读取当前 URL，并重新检查目标是否仍存在、是否禁用。模块不跟踪网页加载结果，也不自动重试或弹出错误窗口。

默认链接文字由控件管理。需要“打开帮助”这样的标签时，使用下面的 OpenUrl 组合，而不是改写内部 Text。

## 自定义外观

### 只复用交互

```rust
bsn! {
    @WidgetryButtonHeadless
    Node { padding: UiRect::all(px(12)) }
    Children [Text("执行操作") WidgetryText]
    on(save)
}
```

Headless 不创建默认背景、内容容器或 Loading。自定义外观可读取 `Hovered`、`Pressed` 和有效 Disabled。不要在同一根再附加原生 Button，以免叠加另一套输入处理。

### 组合 Loading 或 OpenUrl

```rust
bsn! {
    @WidgetryButtonHeadless
    WidgetryButtonLoading::default()
    Node { width: px(160), height: px(40) }
    on(begin_save)
}
```

这个组合具有 Loading 状态和禁用效果，旋转动画、文字和等待光标由自定义 Style 提供。

```rust
bsn! {
    @WidgetryButtonHeadless
    @WidgetryOpenUrl { @url: {"https://bevy.org/learn/".to_owned()} }
    Node { padding: UiRect::all(px(12)) }
    Children [Text("打开帮助") WidgetryText]
}
```

OpenUrl 不添加默认标签或布局。自定义组合在构造时确定，不通过运行期拆除默认 Style 来换肤。
