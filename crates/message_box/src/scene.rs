use crate::lifecycle::{MessageBoxState, forward_activation, handle_message_box_click};
use bevy::app::Propagate;
use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::{ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry_window::{
    WidgetryModalWindow, WidgetryWindowControlsConfig, owned_widgetry_window,
};

/// WidgetryMessageBox 的持久身份，与其 Widgetry WindowRoot 是同一 UI entity。
/// 仅作为 ECS 身份，完整 dialog 必须通过 widgetry_message_box 构造，并注册 WidgetryMessageBoxPlugin；操作系统关闭不发布结果。
#[derive(Component, Default, Clone)]
pub struct WidgetryMessageBox;

/// 私有 Scene 展开入口，由 widgetry_message_box 在同一 root 上附加公开身份与 parent window relationship。
#[derive(SceneComponent, Default, Clone)]
#[scene(MessageBoxProps)]
struct MessageBoxScene;

/// 仅供 SceneComponent 展开的构造输入，展开后不保存为运行期 state。
struct MessageBoxProps {
    /// native window 与 title bar 共享的一次性标题文本。
    title: String,
    /// 底部固定结果 button 组合。
    buttons: WidgetryMessageBoxButtons,
    /// 任意可组合正文；日常调用无需显式 boxing。
    content: Box<dyn SceneList>,
}

/// 只有 Widget 自身的结果 button 携带 action，正文普通 button 没有此语义。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MessageBoxAction(pub WidgetryMessageBoxResult);

/// 异步结果通知；observer 执行期间 root 仍存在，observer command 应用后关闭。
/// 每个 WidgetryMessageBox 最多发布一次；操作系统关闭 native window 不会转换为 Cancel。
#[derive(EntityEvent)]
pub struct WidgetryMessageBoxResultEvent {
    /// WidgetryMessageBox / WindowRoot UI root，不是 native Window entity。
    pub entity: Entity,
    /// 被 click 的结果 button 对应的决议。
    pub result: WidgetryMessageBoxResult,
}

/// 固定的居中结果 button 组，顺序分别为 OK、Yes/No、Yes/No/Cancel。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryMessageBoxButtons {
    Ok,
    YesNo,
    YesNoCancel,
}

/// 用户显式 click 结果 button 后的决议，不含操作系统关闭。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryMessageBoxResult {
    Ok,
    Yes,
    No,
    Cancel,
}

/// 构造固定尺寸、不可 resize 的 non-blocking parent-window modal dialog。
/// parent 必须指向已绑定 Widgetry root 的 native Window，否则创建后清理 child window。
/// content 接收任意 BSN SceneList，普通正文 button 不会产生 WidgetryMessageBox 结果。
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

/// 固定结果 button 自身承载 action，label 作为 button 内容。
fn result_button(result: WidgetryMessageBoxResult) -> impl Scene {
    let label = match result {
        WidgetryMessageBoxResult::Ok => "OK",
        WidgetryMessageBoxResult::Yes => "Yes",
        WidgetryMessageBoxResult::No => "No",
        WidgetryMessageBoxResult::Cancel => "Cancel",
    };
    bsn! {
        @WidgetryButton
        template(move |_| Ok(MessageBoxAction(result)))
        on(forward_activation)
        Node { min_width: px(84), height: px(36), justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [Text(label)]
    }
}

/// 正文与标题继承当前 theme foreground color，button 保留自身 state 配色。
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
    /// 将业务身份直接组合到 owned window root。
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
            owned_widgetry_window(native, controls,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::WidgetryMessageBoxPlugin;
    use bevy_widgetry_button::WidgetryButton;
    use bevy_widgetry_test_utils::scene_app;
    use bevy_widgetry_window::{WidgetryWindowControlsConfig, owned_widgetry_window};

    /// 三种组合生成固定顺序的私有 action，正文普通 button 不带 action。
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
                owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(), bsn_list![], bsn_list![])
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
