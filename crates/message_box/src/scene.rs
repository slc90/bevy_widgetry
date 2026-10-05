use crate::lifecycle::{MessageBoxState, forward_activation, handle_message_box_click};
use bevy::app::Propagate;
use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::{ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry_window::{
    WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowControlsConfig,
    owned_widgetry_window,
};

#[derive(Component, Default, Clone)]
pub struct WidgetryMessageBox;

#[derive(SceneComponent, Default, Clone)]
#[scene(MessageBoxProps)]
struct MessageBoxScene;

struct MessageBoxProps {
    title: String,
    buttons: WidgetryMessageBoxButtons,
    content: Box<dyn SceneList>,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MessageBoxAction(pub WidgetryMessageBoxResult);

#[derive(EntityEvent)]
pub struct WidgetryMessageBoxResultEvent {
    pub entity: Entity,
    pub result: WidgetryMessageBoxResult,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryMessageBoxButtons {
    Ok,
    YesNo,
    YesNoCancel,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryMessageBoxResult {
    Ok,
    Yes,
    No,
    Cancel,
}

pub fn widgetry_message_box(
    parent: Entity,
    title: impl Into<String>,
    buttons: WidgetryMessageBoxButtons,
    content: impl SceneList,
) -> impl Scene {
    let title = title.into();
    let content: Box<dyn SceneList> = Box::new(content);
    bsn! {
        @MessageBoxScene { @title: title, @buttons: buttons, @content: content }
        template(|_| Ok(WidgetryMessageBox))
        template(move |_| Ok(WidgetryModalWindow { parent }))
    }
}

fn result_button(result: WidgetryMessageBoxResult) -> impl Scene {
    let label = match result {
        WidgetryMessageBoxResult::Ok => "OK",
        WidgetryMessageBoxResult::Yes => "Yes",
        WidgetryMessageBoxResult::No => "No",
        WidgetryMessageBoxResult::Cancel => "Cancel",
    };
    bsn! {
        @WidgetryButton
        bevy::input_focus::tab_navigation::TabIndex::default()
        template(move |_| Ok(MessageBoxAction(result)))
        on(forward_activation)
        Node { min_width: px(84), height: px(36), justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text(label)]
    }
}

pub(crate) fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<WidgetryMessageBox>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

impl Default for MessageBoxProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            buttons: WidgetryMessageBoxButtons::Ok,
            content: Box::new(()),
        }
    }
}

impl MessageBoxScene {
    fn scene(props: MessageBoxProps) -> impl Scene {
        let MessageBoxProps {
            title,
            buttons,
            content,
        } = props;
        let native = Window {
            title: title.clone(),
            resolution: (460, 260).into(),
            resizable: false,
            ..default()
        };
        let controls = WidgetryWindowControlsConfig {
            minimize_visible: false,
            maximize_visible: false,
            close_visible: false,
            resizable: false,
        };
        let results: &[WidgetryMessageBoxResult] = match buttons {
            WidgetryMessageBoxButtons::Ok => &[WidgetryMessageBoxResult::Ok],
            WidgetryMessageBoxButtons::YesNo => {
                &[WidgetryMessageBoxResult::Yes, WidgetryMessageBoxResult::No]
            }
            WidgetryMessageBoxButtons::YesNoCancel => &[
                WidgetryMessageBoxResult::Yes,
                WidgetryMessageBoxResult::No,
                WidgetryMessageBoxResult::Cancel,
            ],
        };
        let actions = results
            .iter()
            .map(|&result| bsn! { result_button(result) })
            .collect::<Vec<_>>();
        bsn! {
            template(|_| Ok(MessageBoxState::default()))
            on(handle_message_box_click)
            template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
            owned_widgetry_window(native, controls, WidgetryWindowBackground::Theme,
                bsn_list![(Node { padding: UiRect::left(px(12)), align_items: AlignItems::Center }
                    template(|_| Ok(Pickable::IGNORE))
                    Children [(Text(title) template(|_| Ok(Pickable::IGNORE)))])],
                bsn_list![(
                    Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, min_height: px(0), padding: UiRect::all(px(24)), row_gap: px(20) }
                    Children [
                        (Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, min_height: px(0), row_gap: px(12), overflow: Overflow::clip() } Children [{content}]),
                        (Node { justify_content: JustifyContent::Center, column_gap: px(12), flex_shrink: 0.0 } Children [{actions}]),
                    ]
                )])
        }
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::WidgetryMessageBoxPlugin;
    use bevy_widgetry_button::WidgetryButton;
    use bevy_widgetry_test_utils::scene_app;
    use bevy_widgetry_window::{
        WidgetryWindowBackground, WidgetryWindowControlsConfig, owned_widgetry_window,
    };

    #[test]
    fn result_buttons_have_fixed_order_and_private_actions() {
        for (buttons, expected) in [
            (
                WidgetryMessageBoxButtons::Ok,
                vec![WidgetryMessageBoxResult::Ok],
            ),
            (
                WidgetryMessageBoxButtons::YesNo,
                vec![WidgetryMessageBoxResult::Yes, WidgetryMessageBoxResult::No],
            ),
            (
                WidgetryMessageBoxButtons::YesNoCancel,
                vec![
                    WidgetryMessageBoxResult::Yes,
                    WidgetryMessageBoxResult::No,
                    WidgetryMessageBoxResult::Cancel,
                ],
            ),
        ] {
            let mut app = scene_app();
            app.add_plugins(WidgetryMessageBoxPlugin);
            app.world_mut().commands().spawn_scene(bsn! {
                owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, bsn_list![], bsn_list![])
            });
            app.update();
            let parent = app
                .world_mut()
                .query_filtered::<Entity, With<Window>>()
                .single(app.world())
                .unwrap();
            let root = app.world_mut().commands().spawn_scene(bsn! {
                widgetry_message_box(parent, "Question", buttons, bsn_list![(@WidgetryButton Name("ordinary"))])
            }).id();
            app.update();
            assert!(app.world().get::<WidgetryMessageBox>(root).is_some());
            assert_eq!(
                app.world().get::<WidgetryModalWindow>(root).unwrap().parent,
                parent
            );
            assert!(app.world().get::<ChildOf>(root).is_none());
            let actions: Vec<_> = app
                .world_mut()
                .query::<(&MessageBoxAction, &ChildOf)>()
                .iter(app.world())
                .map(|(action, parent)| (action.0, parent.parent()))
                .collect();
            assert_eq!(actions.len(), expected.len());
            let row = actions[0].1;
            let actual: Vec<_> = app
                .world()
                .get::<Children>(row)
                .unwrap()
                .iter()
                .map(|entity| app.world().get::<MessageBoxAction>(entity).unwrap().0)
                .collect();
            assert_eq!(actual, expected);
            assert!(app.world().get::<UiTargetCamera>(root).is_some());
            let window = app
                .world_mut()
                .query::<&Window>()
                .iter(app.world())
                .find(|w| w.title == "Question")
                .unwrap();
            assert!(!window.resizable);
            let ordinary = app
                .world_mut()
                .query::<(Entity, &Name)>()
                .iter(app.world())
                .find(|(_, name)| name.as_str() == "ordinary")
                .unwrap()
                .0;
            assert!(app.world().get::<MessageBoxAction>(ordinary).is_none());
        }
    }
}
