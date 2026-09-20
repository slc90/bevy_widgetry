use crate::{
    title_bar::{close::CloseButton, maximize::MaximizeButton, minimize::MinimizeButton},
    window_root::{WindowRoot, find_window_root},
};
use bevy::prelude::{ChildOf, Commands, Entity, Window};
use bevy::ui::InteractionDisabled;
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

/// 此 query 集中表达 style 同步所需的数据访问与 entity filter 条件。
type ChangedControlStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Hovered, Has<Pressed>, &'static mut BackgroundColor),
    (
        Or<(With<MinimizeButton>, With<MaximizeButton>)>,
        Or<(Changed<Hovered>, Added<Pressed>)>,
    ),
>;

/// 此 query 集中表达 style 同步所需的数据访问与 entity filter 条件。
type ControlStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Hovered, Has<Pressed>, &'static mut BackgroundColor),
    Or<(With<MinimizeButton>, With<MaximizeButton>)>,
>;

/// 统一系统按钮的 hit area 和内容居中的 layout。
pub(super) fn window_control_button_node() -> Node {
    Node {
        width: px(46),
        height: percent(100),
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    }
}

/// 普通系统按钮的 pressed 优先于 hover，无交互时使用透明背景。
fn window_control_background(hovered: bool, pressed: bool) -> Color {
    if pressed {
        Color::srgba(1.0, 1.0, 1.0, 0.14)
    } else if hovered {
        Color::srgba(1.0, 1.0, 1.0, 0.08)
    } else {
        Color::srgba(0.0, 0.0, 0.0, 0.0)
    }
}

/// 仅刷新 minimize 和 maximize button，close button 采用独立配色。
pub(super) fn update_window_control_style_changed(mut query: ChangedControlStyleQuery<'_, '_>) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = window_control_background(hovered.0, pressed);
    }
}

/// pressed 移除时恢复系统按钮背景，保留仍有效的 hover state。
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

/// native button 配置是唯一 state 来源，运行时修改后同步 Bevy 的 InteractionDisabled marker。
pub(super) fn sync_enabled_buttons(
    buttons: Query<
        (
            Entity,
            Has<MinimizeButton>,
            Has<MaximizeButton>,
            Has<InteractionDisabled>,
        ),
        Or<(
            With<MinimizeButton>,
            With<MaximizeButton>,
            With<CloseButton>,
        )>,
    >,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    windows: Query<&Window>,
    mut commands: Commands,
) {
    for (entity, minimize, maximize, disabled) in &buttons {
        let Some(root) = find_window_root(entity, &parents, &roots) else {
            continue;
        };
        let Ok(window) = windows.get(root.target_window) else {
            continue;
        };
        let enabled = if minimize {
            window.enabled_buttons.minimize
        } else if maximize {
            window.enabled_buttons.maximize
        } else {
            window.enabled_buttons.close
        };
        if enabled && disabled {
            commands.entity(entity).remove::<InteractionDisabled>();
        } else if !enabled && !disabled {
            commands.entity(entity).insert(InteractionDisabled);
        }
    }
}
