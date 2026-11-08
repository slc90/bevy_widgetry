const ACTION_MIN_WIDTH: f32 = 84.0;
const ACTION_HEIGHT: f32 = 36.0;
const NATIVE_WINDOW_SIZE: (u32, u32) = (460, 260);
const TITLE_LEFT_PADDING: f32 = 12.0;
const BODY_PADDING: f32 = 24.0;
const BODY_GAP: f32 = 20.0;
const CONTENT_GAP: f32 = 12.0;
const ACTIONS_COLUMN_GAP: f32 = 12.0;
const ACTIONS_ROW_GAP: f32 = 8.0;

use crate::lifecycle::{MessageBoxState, forward_activation, handle_message_box_click};
use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_theme::WidgetryTheme;
use bevy_widgetry_window::{
    WidgetryModalWindow, WidgetryWindowBackground, WidgetryWindowControlsConfig,
    owned_widgetry_window,
};

#[derive(Component, Default, Clone)]
#[require(crate::colors::ColorState, crate::style::MessageBoxStyle)]
pub struct WidgetryMessageBox;

#[derive(Component, Default, Clone)]
pub(crate) struct MessageBoxBody;

#[derive(SceneComponent, Default, Clone)]
#[scene(MessageBoxProps)]
struct MessageBoxScene;

struct MessageBoxProps {
    colors: crate::WidgetryMessageBoxColorOverrides,
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
    colors: crate::WidgetryMessageBoxColorOverrides,
    content: impl SceneList,
) -> impl Scene {
    let title = title.into();
    let content: Box<dyn SceneList> = Box::new(content);
    bsn! {
        @MessageBoxScene { @title: title, @buttons: buttons, @colors: colors, @content: content }
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
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {Text(label) bevy_widgetry_core::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        bevy::input_focus::tab_navigation::TabIndex::default()
        template(move |_| Ok(MessageBoxAction(result)))
        on(forward_activation)
        Node { min_width: px(ACTION_MIN_WIDTH), height: px(ACTION_HEIGHT), justify_content: JustifyContent::Center, align_items: AlignItems::Center }

    }
}

impl Default for MessageBoxProps {
    fn default() -> Self {
        Self {
            colors: default(),
            title: String::new(),
            buttons: WidgetryMessageBoxButtons::Ok,
            content: Box::new(()),
        }
    }
}

impl MessageBoxScene {
    fn scene(props: MessageBoxProps) -> impl Scene {
        let MessageBoxProps {
            colors,
            title,
            buttons,
            content,
        } = props;
        let native = Window {
            title: title.clone(),
            resolution: NATIVE_WINDOW_SIZE.into(),
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
            .map(|&result| bsn! { @result_button(result) })
            .collect::<Vec<_>>();
        bsn! {
            template(move |context| colors.clone().initial().map_err(|failure| failure.with_context(bevy_widgetry_diagnostics::ErrorFrame::new("message_box.colors.initialize").field("message_box", context.entity.id())).into_bevy_error()))
            template(|_| Ok(MessageBoxState::default()))
            on(handle_message_box_click)
            template(|context| Ok(ResolvedForeground(context.resource::<WidgetryTheme>().message_box.body.normal.foreground)))
            @owned_widgetry_window(native, controls, WidgetryWindowBackground::Theme, Default::default(),
                bsn_list!{Node { padding: UiRect::left(px(TITLE_LEFT_PADDING)), align_items: AlignItems::Center }
                    template(|_| Ok(Pickable::IGNORE))
                    Children [bevy_widgetry_window::WidgetryWindowTitle bevy_widgetry_core::text::WidgetryText template(|_| Ok(Pickable::IGNORE))]},
                bsn_list!{
                    MessageBoxBody
                    ResolvedForeground
                    BackgroundColor
                    Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, min_height: px(0), padding: UiRect::all(px(BODY_PADDING)), row_gap: px(BODY_GAP) }
                    Children [
                        @bevy_widgetry_scroll_area::WidgetryScrollArea { @children: {Some(Box::new(bsn_list! {
                            Node {width:percent(100),max_width:percent(100),flex_shrink:0.0,flex_direction:FlexDirection::Column,row_gap:px(CONTENT_GAP)} Children [{content}]
                        }) as Box<dyn SceneList>)} }
                        Node { width:percent(100),flex_grow:1.0,min_width:px(0),min_height:px(0) }--
                        Node { justify_content: JustifyContent::Center, column_gap: px(ACTIONS_COLUMN_GAP),row_gap:px(ACTIONS_ROW_GAP),flex_wrap:FlexWrap::Wrap, flex_shrink: 0.0 } Children [{actions}]
                    ]
                })
        }
    }
}

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
                @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), WidgetryWindowBackground::Theme, Default::default(),  bsn_list!{}, bsn_list!{})
            });
            app.update();
            let parent = app
                .world_mut()
                .query_filtered::<Entity, With<Window>>()
                .single(app.world())
                .unwrap();
            let root = app.world_mut().commands().spawn_scene(bsn! {
                @widgetry_message_box(parent, "Question", buttons, Default::default(),  bsn_list!{@WidgetryButton Name("ordinary")})
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
