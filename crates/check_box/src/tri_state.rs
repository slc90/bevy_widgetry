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
use bevy_widgetry_log::widgetry_error;

/// 三态 CheckBox 的唯一真实 state，默认未选中。
/// 只读查询此 immutable Component；runtime 更新使用 WidgetryTriStateCheckbox 的 set_state / cycle_state。
/// 直接替换或移除 Component 属于 ECS 结构操作，不承诺 Widget API 的变化通知。
#[derive(Component, Default, Clone, Copy, Debug, PartialEq, Eq, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub enum WidgetryCheckState {
    /// 未选中。
    #[default]
    Unchecked,
    /// 已选中。
    Checked,
    /// 部分选中。
    Indeterminate,
}

/// 使用独立三态 state 的 CheckBox；UI 与程序 API 均先提交 state，再发 ValueChange<WidgetryCheckState>。
/// 需注册 WidgetryCheckBoxPlugin；调用方通过 BSN Children 添加 label。
/// root 上的 observer 可读取已提交的 enum；source=root、is_final=true，不区分输入来源。
/// 同值不通知，style / Accessibility projection 在后续 system 同步；通知不作为 state 更新请求。
#[derive(SceneComponent, Default, Clone)]
#[require(crate::style::StyleDiagnostics)]
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
fn write_state(
    world: &mut World,
    entity: Entity,
    target: Option<WidgetryCheckState>,
) -> Result<(), BevyError> {
    if world.get::<WidgetryTriStateCheckbox>(entity).is_none() {
        widgetry_error!(?entity, "三态 CheckBox 更新目标不存在或不是三态 Widget");
        return Err(BevyError::error(
            "三态 CheckBox 更新目标不存在或不是三态 Widget",
        ));
    }
    let Some(current) = world.get::<WidgetryCheckState>(entity).copied() else {
        widgetry_error!(?entity, "三态 CheckBox 缺失必需 state");
        return Err(BevyError::error("三态 CheckBox 缺失必需 state"));
    };
    let next = target.unwrap_or_else(|| next_check_state(current));
    if next != current {
        world.entity_mut(entity).insert(next);
        world.trigger(ValueChange {
            source: entity,
            value: next,
            is_final: true,
        });
    }
    Ok(())
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
    let Ok((_state, disabled, pressed, activate_on_press)) = query.get(event.entity) else {
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
            WidgetryTriStateCheckbox::cycle_state(&mut commands, event.entity);
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
    let Ok((_state, disabled)) = query.get(event.entity) else {
        return;
    };
    event.propagate(false);
    if !disabled {
        WidgetryTriStateCheckbox::cycle_state(&mut commands, event.entity);
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
    let Ok(_state) = query.get(event.focused_entity) else {
        return;
    };
    let input = &event.event().input;
    if input.state == ButtonState::Pressed
        && !input.repeat
        && matches!(input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        event.propagate(false);
        WidgetryTriStateCheckbox::cycle_state(&mut commands, event.focused_entity);
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

    /// 排队设置 state，disabled 仍允许；执行时先提交再通知，同值不写入、不通知。
    /// 目标失效、不是三态 Widget 或缺失 state 时记录 ERROR，并以 Severity::Error 交给宿主 handler。
    pub fn set_state(commands: &mut Commands, entity: Entity, state: WidgetryCheckState) {
        commands.queue(move |world: &mut World| write_state(world, entity, Some(state)));
    }

    /// 排队进入下一 state；执行时读取当前值，连续调用可连续循环，每次提交后通知。
    /// 顺序为 Unchecked → Checked → Indeterminate → Unchecked；错误与 disabled 语义同 set_state。
    pub fn cycle_state(commands: &mut Commands, entity: Entity) {
        commands.queue(move |world: &mut World| write_state(world, entity, None));
    }
}

// 测试 module 中的断言用于验证 contract，生产代码仍禁止。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    // 每个合法三态输入都有确定的后继，不依赖 observer 或程序化 queue。
    #[test]
    fn next_state_follows_three_state_cycle() {
        for (from, to) in [
            (WidgetryCheckState::Unchecked, WidgetryCheckState::Checked),
            (
                WidgetryCheckState::Checked,
                WidgetryCheckState::Indeterminate,
            ),
            (
                WidgetryCheckState::Indeterminate,
                WidgetryCheckState::Unchecked,
            ),
        ] {
            assert_eq!(next_check_state(from), to);
        }
    }
}
