# Widgetry 文档

Widgetry 基于 Bevy 0.20 提供控件、数据视图、绘图与桌面应用基础能力。

需要使用某项功能时，从 [功能目录](features/README.md) 查找；需要维护或扩展实现时，先读 [整体设计](design/architecture.md)，再进入对应的模块与机制。

## 从一个按钮开始

应用通过 Git 依赖使用库，并安装需要的控件插件：

```toml
[dependencies]
bevy = { version = "=0.20.0", features = ["bevy_scene", "bevy_ui_widgets", "bevy_input_focus", "ui_picking"] }
bevy_widgetry = { git = "https://github.com/slc90/bevy_widgetry", default-features = false, features = ["button"] }
```

```rust
use bevy::{prelude::*, ui_widgets::Activate};
use bevy_widgetry::{
    button::{WidgetryButton, WidgetryButtonPlugin},
    scene::WidgetrySceneCommandsExt,
    text::WidgetryText,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, WidgetryButtonPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.spawn_scene_with_error_handler(bsn! {
        Node {
            width: percent(100), height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center
        }
        Children [
            @WidgetryButton {
                @children: {Some(Box::new(bsn_list! {
                    Text("Confirm") WidgetryText
                }) as Box<dyn SceneList>)}
            }
            on(activated)
        ]
    });
}

fn activated(event: On<Activate>) {
    info!(entity = ?event.entity, "按钮已激活");
}
```

更完整的内容布局、Loading、颜色和 URL 用法见 [Button](features/button.md)。独立原生窗口与透明呈现的装配见 [Window](features/window.md)，不需要透明窗口时不必引入这套初始化。

## 按阅读目的选择入口

| 入口 | 阅读内容 |
| --- | --- |
| [功能说明](features/README.md) | 能做什么、如何创建、设置、查询与响应事件 |
| [整体设计](design/architecture.md) | Widgetry 与 Bevy 的关系，以及库如何组织与组合 |
| [模块核心设计](design/modules/README.md) | 单个能力的状态权威、结构与生命周期 |
| [跨模块机制](design/mechanisms/README.md) | 多个能力如何共同处理输入、禁用、外观、诊断和资源 |

## 示例中的上下文

各功能页的 Rust 片段按任务独立展开。通用类型通过 `bevy::prelude::*` 引入；页面给出该能力需要的 Widgetry 导入。

`app` 表示正在装配的 `App`，`world` 表示 `&mut World`，`commands` 表示当前系统的 `Commands`。`root`、`button`、`view` 等实体参数指对应控件的根或页面明确说明的目标，不是随意的内容子实体。示例中的文件、ICO 和图集由调用应用提供。

带 `?` 的片段放在返回相应 `Result` 的函数中。领域失败在 System、Observer 或 Commands 等 Bevy 执行边界转换为 `BevyError`；默认错误策略与最终用户提示由应用按需要配置。

默认控件插件通常会确保自身需要的 Headless 插件已经安装。自定义外观章节中的 Headless-only 装配是另一种选择，不是在默认插件之后再重复添加相同插件。中文内容可通过 Core 的默认字体能力或应用自有字体显示。

BSN 表达构造，运行状态通过组件和公开操作维护。`_in_world` 通常表示同步入口，Commands 入口在实际执行时提交。状态已提交与布局、视觉已刷新是两个时刻，具体通知语义由各能力说明。
