# 颜色主题

Theme 提供完整的控件配色与运行时切换。它只管理颜色，不改变布局、尺寸、字体或动画，也不选择光标图片主题。

## 主题预设

### 使用默认配色

```rust
use bevy::prelude::*;
use bevy_widgetry::theme::{WidgetryThemePlugin, WidgetryTheme, WIDGETRY_DARK_THEME};

app.add_plugins(WidgetryThemePlugin);
```

没有预先指定主题时采用 Dark。各控件插件会确保需要的主题能力已安装；宿主显式安装时只需一次。

### 编译可选预设

```toml
[dependencies]
bevy_widgetry = { git = "https://github.com/slc90/bevy_widgetry", features = ["light", "pink_dream"] }
```

启用后可使用 `WIDGETRY_LIGHT_THEME` 和 `WIDGETRY_PINK_DREAM_THEME`。关闭对应 Feature 时，常量本身不参与编译，不会在运行时自动替换成 Dark。完整 `WidgetryTheme` 的字段结构不随预设开关改变。

## 运行时切换

### 同步提交主题

```rust
let changed = WidgetryTheme::set_in_world(world, WIDGETRY_DARK_THEME)
    .map_err(|failure| failure.into_bevy_error())?;
```

Setter 校验整套颜色后提交。实际变化返回 `true`，合法同值返回 `false`；非法颜色保留原主题。需要延迟执行时使用：

```rust
WidgetryTheme::set(&mut commands, WIDGETRY_DARK_THEME);
```

Commands 入口在实际执行时校验和提交，错误交给宿主处理。

### 接收变化通知

```rust
use bevy_widgetry::theme::WidgetryThemeChanged;

fn theme_changed(_event: On<WidgetryThemeChanged>, theme: Res<WidgetryTheme>) {
    info!(foreground = ?theme.button.normal.foreground, "已提交新主题");
}

app.add_observer(theme_changed);
```

通知发出时 Resource 已更新。同值设置不通知。颜色消费者在自己的呈现阶段更新，不需要通过销毁控件来换肤。

## 自定义配色

### 从完整预设修改

```rust
let mut theme = WIDGETRY_DARK_THEME;
theme.button.normal.background = Color::srgb(0.12, 0.24, 0.42);
theme.button.hovered.background = Color::srgb(0.16, 0.32, 0.56);
theme.button.normal.foreground = Color::WHITE;

WidgetryTheme::set_in_world(world, theme)
    .map_err(|failure| failure.into_bevy_error())?;
```

复制预设可以保留其余控件的完整配置，不必从零填写所有颜色。主题名称、选择菜单和持久化方式属于应用，库接收的是完整配色数据。

颜色分量须有限，Alpha 在合法范围内；部分循环调色板还有自身的邻接和数量约束。校验失败不会只应用一部分字段。

### 保留控件的局部覆盖

主题与控件 ColorOverrides 是两层设置。主题提供默认值，控件仅对明确填写的字段覆盖。覆盖字段不随主题切换变化，未覆盖字段继续跟随新主题。

要恢复全局配色，调用该控件颜色类型的 `clear` 或 `clear_in_world`。普通按钮示例见 [Button](button.md) 的外观与颜色章节。

## 组合控件配色

组合控件可以统一提供内部部件的颜色。对这些已由组合层管理的部件直接写入另一套局部覆盖，可能被明确拒绝；应使用组合控件公开的配色入口。

默认 Style 只管理自身拥有的输出，不会因为全局主题变化而重新构造调用方的业务内容。原生 Text 与 Widgetry 标记文本的颜色行为不同，独立文字的用法见 [Core](core.md)。
