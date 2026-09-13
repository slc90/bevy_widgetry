use crate::lifecycle::{MessageBoxState, forward_activation, handle_message_box_click};
use bevy::app::Propagate;
use bevy::prelude::*;
use bevy_widgetry_button::StyledButton;
use bevy_widgetry_core::{ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry_window::{ModalWindow, WindowControlsConfig, owned_window};

/// MessageBox 的持久身份，与其 Widgetry WindowRoot 是同一 UI 实体。
/// 仅作为 ECS 身份，完整对话框必须通过 message_box 构造，并注册 MessageBoxPlugin；系统关闭不发布结果。
#[derive(Component, Default, Clone)]
pub struct MessageBox;

/// 私有场景展开入口，由 message_box 在同一根上附加公开身份与父窗口关系。
#[derive(SceneComponent, Default, Clone)]
#[scene(MessageBoxProps)]
struct MessageBoxScene;

/// 仅供 SceneComponent 展开的构造输入，展开后不保存为运行期状态。
struct MessageBoxProps {
    /// 原生窗口与标题栏共享的一次性标题文本。
    title: String,
    /// 底部固定结果按钮组合。
    buttons: MessageBoxButtons,
    /// 任意可组合正文；日常调用无需显式装箱。
    content: Box<dyn SceneList>,
}

/// 只有控件自身的结果按钮携带 action，正文普通按钮没有此语义。
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MessageBoxAction(pub MessageBoxResult);

/// 异步结果通知；observer 执行期间根仍存在，observer 命令应用后关闭。
/// 每个 MessageBox 最多发布一次；原生系统关闭不会转换为 Cancel。
#[derive(EntityEvent)]
pub struct MessageBoxResultEvent {
    /// MessageBox / WindowRoot UI 根，不是原生 Window 实体。
    pub entity: Entity,
    /// 被点击结果按钮对应的决议。
    pub result: MessageBoxResult,
}

/// 固定的居中结果按钮组，顺序分别为 OK、Yes/No、Yes/No/Cancel。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageBoxButtons {
    Ok,
    YesNo,
    YesNoCancel,
}

/// 用户显式点击结果按钮后的决议，不含系统关闭。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageBoxResult {
    Ok,
    Yes,
    No,
    Cancel,
}

/// 构造固定尺寸、不可缩放的非阻塞父窗口模态对话框。
/// parent 必须指向已绑定 Widgetry 根的原生 Window，否则创建后清理子窗口。
/// content 接收任意 BSN SceneList，普通正文按钮不会产生 MessageBox 结果。
pub fn message_box(
    parent: Entity,
    title: impl Into<String>,
    buttons: MessageBoxButtons,
    content: impl SceneList,
) -> impl Scene {
    let title = title.into();
    let content: Box<dyn SceneList> = Box::new(content);
    bsn! {
        @MessageBoxScene { @title: title, @buttons: buttons, @content: content }
        template(|_| Ok(MessageBox))
        template(move |_| Ok(ModalWindow { parent }))
    }
}

/// 固定结果按钮自身承载 action，文字不拦截拾取以保持原始 target 为按钮。
fn result_button(result: MessageBoxResult) -> impl Scene {
    let label = match result {
        MessageBoxResult::Ok => "OK",
        MessageBoxResult::Yes => "Yes",
        MessageBoxResult::No => "No",
        MessageBoxResult::Cancel => "Cancel",
    };
    bsn! {
        template(|_| Ok(StyledButton))
        template(move |_| Ok(MessageBoxAction(result)))
        on(forward_activation)
        Node { min_width: px(84), height: px(36), padding: UiRect::axes(px(12), px(6)), border: UiRect::all(px(1)), justify_content: JustifyContent::Center, align_items: AlignItems::Center }
        Children [(Text(label) template(|_| Ok(Pickable::IGNORE)))]
    }
}

/// 正文与标题继承当前主题前景色，按钮保留自身状态配色。
pub(crate) fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<MessageBox>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

impl Default for MessageBoxProps {
    fn default() -> Self {
        Self {
            title: String::new(),
            buttons: MessageBoxButtons::Ok,
            content: Box::new(()),
        }
    }
}

impl MessageBoxScene {
    /// 将业务身份直接组合到 owned window 根。
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
        let controls = WindowControlsConfig {
            minimize_visible: false,
            maximize_visible: false,
            close_visible: false,
            resizable: false,
        };
        let results: &[MessageBoxResult] = match buttons {
            MessageBoxButtons::Ok => &[MessageBoxResult::Ok],
            MessageBoxButtons::YesNo => &[MessageBoxResult::Yes, MessageBoxResult::No],
            MessageBoxButtons::YesNoCancel => &[
                MessageBoxResult::Yes,
                MessageBoxResult::No,
                MessageBoxResult::Cancel,
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
            owned_window(native, controls,
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
    use crate::MessageBoxPlugin;
    use bevy_widgetry_button::StyledButton;
    use bevy_widgetry_test_utils::scene_app;
    use bevy_widgetry_window::{WindowControlsConfig, owned_window};

    /// 三种组合生成固定顺序的私有 action，正文普通按钮不带 action。
    #[test]
    fn result_buttons_have_fixed_order_and_private_actions() {
        for (buttons, expected) in [
            (MessageBoxButtons::Ok, vec![MessageBoxResult::Ok]),
            (
                MessageBoxButtons::YesNo,
                vec![MessageBoxResult::Yes, MessageBoxResult::No],
            ),
            (
                MessageBoxButtons::YesNoCancel,
                vec![
                    MessageBoxResult::Yes,
                    MessageBoxResult::No,
                    MessageBoxResult::Cancel,
                ],
            ),
        ] {
            let mut app = scene_app();
            app.add_plugins(MessageBoxPlugin);
            app.world_mut().commands().spawn_scene(bsn! {
                owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
            });
            app.update();
            let parent = app
                .world_mut()
                .query_filtered::<Entity, With<Window>>()
                .single(app.world())
                .unwrap();
            let root = app.world_mut().commands().spawn_scene(bsn! {
                message_box(parent, "Question", buttons, bsn_list![(template(|_| Ok(StyledButton)) Name("ordinary"))])
            }).id();
            app.update();
            assert!(app.world().get::<MessageBox>(root).is_some());
            assert_eq!(app.world().get::<ModalWindow>(root).unwrap().parent, parent);
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
