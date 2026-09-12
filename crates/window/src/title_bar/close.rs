use crate::{
    title_bar::controls::window_control_button_node,
    window_root::{WindowRoot, find_window_root},
};
use bevy::{
    color::Color,
    ecs::{
        component::Component,
        hierarchy::ChildOf,
        lifecycle::RemovedComponents,
        message::MessageWriter,
        observer::On,
        query::{Added, Changed, Has, Or, With},
        system::Query,
    },
    picking::hover::Hovered,
    ui::{BackgroundColor, Node, Pressed},
    ui_widgets::{Activate, Button},
    window::{Window, WindowCloseRequested},
};

/// 将激活操作转换为关联窗口的关闭请求。
#[derive(Component)]
#[require(
    Button,
    Hovered,
    Node = window_control_button_node(),
    BackgroundColor,
)]
pub(super) struct CloseButton;

/// 此查询集中表达样式同步所需的数据访问与实体过滤条件。
type ChangedCloseStyleQuery<'w, 's> = Query<
    'w,
    's,
    (&'static Hovered, Has<Pressed>, &'static mut BackgroundColor),
    (With<CloseButton>, Or<(Changed<Hovered>, Added<Pressed>)>),
>;

/// 向所属真实窗口发送关闭请求，保留上层处理关闭策略的机会。
pub(super) fn on_close(
    event: On<Activate>,
    buttons: Query<(), With<CloseButton>>,
    parents: Query<&ChildOf>,
    roots: Query<&WindowRoot>,
    windows: Query<&Window>,
    mut close_requests: MessageWriter<WindowCloseRequested>,
) {
    let Ok(()) = buttons.get(event.entity) else {
        return;
    };

    let Some(root) = find_window_root(event.entity, &parents, &roots) else {
        return;
    };

    if !windows
        .get(root.target_window)
        .is_ok_and(|window| window.enabled_buttons.close)
    {
        return;
    }

    close_requests.write(WindowCloseRequested {
        window: root.target_window,
    });
}

/// 关闭按钮以红色区分危险操作，按压时使用更深背景。
fn close_button_background(hovered: bool, pressed: bool) -> Color {
    if pressed {
        Color::srgb_u8(180, 30, 30)
    } else if hovered {
        Color::srgb_u8(196, 43, 28)
    } else {
        Color::NONE
    }
}

/// 根据当前悬停与按压状态刷新关闭按钮背景。
pub(super) fn update_close_button_style_changed(mut query: ChangedCloseStyleQuery<'_, '_>) {
    for (hovered, pressed, mut background) in &mut query {
        background.0 = close_button_background(hovered.0, pressed);
    }
}

/// 按压移除后读取当前悬停状态，恢复关闭按钮背景。
pub(super) fn update_close_button_style_released(
    mut removed_pressed: RemovedComponents<Pressed>,
    mut query: Query<(&Hovered, Has<Pressed>, &mut BackgroundColor), With<CloseButton>>,
) {
    for entity in removed_pressed.read() {
        let Ok((hovered, pressed, mut background)) = query.get_mut(entity) else {
            continue;
        };

        background.0 = close_button_background(hovered.0, pressed);
    }
}
