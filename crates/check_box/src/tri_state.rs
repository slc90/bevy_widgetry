use crate::indicator::checkbox_indicator_scene;
use accesskit::{Role, Toggled};
use bevy::a11y::AccessibilityNode;
use bevy::app::Propagate;
use bevy::input::{
    ButtonState,
    keyboard::{KeyCode, KeyboardInput},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus, InputFocusVisible};
use bevy::picking::events::{Cancel, Click, DragEnd, Pointer, Press, Release};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{BackgroundColor, BorderColor, InteractionDisabled, Pressed};
use bevy::ui_widgets::{ActivateOnPress, ValueChange};
use bevy_widgetry_core::ForegroundColor;

/// 三态 CheckBox 的唯一真实 state，默认未选中。
#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum WidgetryCheckState {
    /// 未选中。
    #[default]
    Unchecked,
    /// 已选中。
    Checked,
    /// 部分选中。
    Indeterminate,
}

/// 使用独立三态 state 的 CheckBox；用户交互发出最终 ValueChange，程序化 API 静默修改 state。
/// 需注册 WidgetryCheckBoxPlugin；调用方通过 BSN Children 添加 label。
#[derive(SceneComponent, Default, Clone)]
pub struct WidgetryTriStateCheckbox;

/// 所有输入路径共用的三态顺序。
fn next_check_state(state: WidgetryCheckState) -> WidgetryCheckState {
    match state {
        WidgetryCheckState::Unchecked => WidgetryCheckState::Checked,
        WidgetryCheckState::Checked => WidgetryCheckState::Indeterminate,
        WidgetryCheckState::Indeterminate => WidgetryCheckState::Unchecked,
    }
}

/// 在 Commands queue 执行时检查实体身份和当前 state，再写入目标 state。
fn write_state(world: &mut World, entity: Entity, target: Option<WidgetryCheckState>) {
    let Some(current) = world.get::<WidgetryCheckState>(entity).copied() else {
        return;
    };
    if world.get::<WidgetryTriStateCheckbox>(entity).is_none() {
        return;
    }
    let next = target.unwrap_or_else(|| next_check_state(current));
    if next != current {
        world.entity_mut(entity).insert(next);
    }
}

/// 用户交互的 ValueChange 自更新；只接受三态 Widget 的来源。
pub(crate) fn widgetry_tri_state_checkbox_self_update(
    event: On<ValueChange<WidgetryCheckState>>,
    mut commands: Commands,
    query: Query<(), With<WidgetryTriStateCheckbox>>,
) {
    if query.contains(event.source) {
        let entity = event.source;
        let value = event.value;
        commands.queue(move |world: &mut World| write_state(world, entity, Some(value)));
    }
}

/// 在 Press 时保持官方 Checkbox 的 focus 语义，并按 ActivateOnPress 决定是否立即循环。
pub(crate) fn on_press(
    mut event: On<Pointer<Press>>,
    query: Query<
        (
            &WidgetryCheckState,
            Has<InteractionDisabled>,
            Has<Pressed>,
            Has<ActivateOnPress>,
        ),
        With<WidgetryTriStateCheckbox>,
    >,
    focus: Option<ResMut<InputFocus>>,
    focus_visible: Option<ResMut<InputFocusVisible>>,
    mut commands: Commands,
) {
    let Ok((state, disabled, pressed, activate_on_press)) = query.get(event.entity) else {
        return;
    };
    if let Some(mut focus) = focus {
        focus.set(event.entity, FocusCause::Pressed);
    }
    if let Some(mut visible) = focus_visible {
        visible.0 = false;
    }
    event.propagate(false);
    if !disabled && !pressed {
        commands.entity(event.entity).insert(Pressed);
        if activate_on_press {
            commands.trigger(ValueChange {
                source: event.entity,
                value: next_check_state(*state),
                is_final: true,
            });
        }
    }
}

/// 默认在 Click 时循环；ActivateOnPress 已在 Press 处理，避免二次循环。
pub(crate) fn on_click(
    mut event: On<Pointer<Click>>,
    query: Query<
        (&WidgetryCheckState, Has<InteractionDisabled>),
        (With<WidgetryTriStateCheckbox>, Without<ActivateOnPress>),
    >,
    mut commands: Commands,
) {
    let Ok((state, disabled)) = query.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if !disabled {
        commands.trigger(ValueChange {
            source: event.entity,
            value: next_check_state(*state),
            is_final: true,
        });
    }
}

/// Release 清除 pressed state。
pub(crate) fn on_release(
    mut event: On<Pointer<Release>>,
    query: Query<(), With<WidgetryTriStateCheckbox>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

/// DragEnd 清除 pressed state。
pub(crate) fn on_drag_end(
    mut event: On<Pointer<DragEnd>>,
    query: Query<(), With<WidgetryTriStateCheckbox>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

/// Cancel 清除 pressed state。
pub(crate) fn on_cancel(
    mut event: On<Pointer<Cancel>>,
    query: Query<(), With<WidgetryTriStateCheckbox>>,
    mut commands: Commands,
) {
    if query.contains(event.entity) {
        event.propagate(false);
        commands.entity(event.entity).remove::<Pressed>();
    }
}

/// 仅首次 Space 或 Enter Press 发出一次用户 ValueChange。
pub(crate) fn on_key(
    mut event: On<FocusedInput<KeyboardInput>>,
    query: Query<
        &WidgetryCheckState,
        (With<WidgetryTriStateCheckbox>, Without<InteractionDisabled>),
    >,
    mut commands: Commands,
) {
    let Ok(state) = query.get(event.focused_entity) else {
        return;
    };
    let input = &event.event().input;
    if input.state == ButtonState::Pressed
        && !input.repeat
        && matches!(input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        event.propagate(false);
        commands.trigger(ValueChange {
            source: event.focused_entity,
            value: next_check_state(*state),
            is_final: true,
        });
    }
}

/// 从真实 state 同步 Accessibility，覆盖用户和程序化写入。
pub(crate) fn sync_accessibility(
    mut query: Query<(&WidgetryCheckState, &mut AccessibilityNode), Changed<WidgetryCheckState>>,
) {
    for (state, mut node) in &mut query {
        node.set_toggled(match state {
            WidgetryCheckState::Unchecked => Toggled::False,
            WidgetryCheckState::Checked => Toggled::True,
            WidgetryCheckState::Indeterminate => Toggled::Mixed,
        });
    }
}

impl WidgetryTriStateCheckbox {
    /// 展开三态 Scene，state 与 Accessibility 由当前 entity 承载。
    fn scene() -> impl Scene {
        bsn! {
            template(|_| Ok(WidgetryCheckState::Unchecked))
            template(|_| Ok(AccessibilityNode(accesskit::Node::new(Role::CheckBox))))
            Hovered(false)
            TabIndex(-1)
            Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, column_gap: px(6), min_height: px(24) }
            BackgroundColor
            BorderColor
            template(|_| Ok(Propagate(ForegroundColor::default())))
            Children [checkbox_indicator_scene()]
        }
    }

    /// 静默设置 state；同值、无效 entity 和非三态 entity 均保持不变，disabled 仍允许。
    pub fn set_state(commands: &mut Commands, entity: Entity, state: WidgetryCheckState) {
        commands.queue(move |world: &mut World| write_state(world, entity, Some(state)));
    }

    /// 静默进入下一 state；在 queue 执行时读取当前值，连续调用可连续循环。
    pub fn cycle_state(commands: &mut Commands, entity: Entity) {
        commands.queue(move |world: &mut World| write_state(world, entity, None));
    }
}
