use crate::title_bar::{maximize::MaximizeButton, minimize::MinimizeButton};
use bevy::{
    color::Color,
    ecs::{
        lifecycle::RemovedComponents,
        query::{Added, Changed, Has, Or, With},
        system::Query,
    },
    picking::hover::Hovered,
    ui::{AlignItems, BackgroundColor, JustifyContent, Node, Pressed, percent, px},
    utils::default,
};

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type ChangedControlStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Hovered, Has<Pressed>, &'static mut BackgroundColor),
    (
        Or<(With<MinimizeButton>, With<MaximizeButton>)>,
        Or<(Changed<Hovered>, Added<Pressed>)>,
    ),
>;

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type ControlStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Hovered, Has<Pressed>, &'static mut BackgroundColor),
    Or<(With<MinimizeButton>, With<MaximizeButton>)>,
>;

/// 统一系统按钮命中区域和内容居中布局。
pub(super) fn window_control_button_node() -> Node {
    Node {
        width: px(46),
        height: percent(100),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

/// 普通系统按钮按压优先于悬停，无交互时使用透明背景。
fn window_control_background(hovered: bool, pressed: bool) -> Color {
    if pressed {
        Color::srgba(1.0, 1.0, 1.0, 0.14)
    } else if hovered {
        Color::srgba(1.0, 1.0, 1.0, 0.08)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
    }
}

/// 仅刷新最小化和最大化按钮，关闭按钮采用独立配色。
pub(super) fn update_window_control_style_changed(mut query: ChangedControlStyleQuery<'_, '_>) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = window_control_background(hovered.0, pressed);
    }
}

/// 按压移除时恢复系统按钮背景，保留仍有效的悬停状态。
pub(super) fn update_window_control_style_released(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut query: ControlStyleQuery<'_, '_>,
) {
    for entity in removed_pressed.read() {
        let Ok((hovered, pressed, mut background)) = query.get_mut(entity) else {
            continue;
        };

        background.0 = window_control_background(hovered.0, pressed);
    }
}
