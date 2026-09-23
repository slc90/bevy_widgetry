use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::check_box::{WidgetryCheckBox, WidgetryCheckState, WidgetryTriStateCheckbox};
use bevy_widgetry::style::{ForegroundColor, ThemeChanged, ThemeMode};

/// 刷新 Gallery 自有分组标题，不接管 CheckBox 样式。
pub(crate) struct CheckBoxDemoPlugin;

/// 标记页面的 theme 传播 root。
#[derive(Component)]
struct CheckBoxDemo;

/// 标记 Gallery 中初始部分选中的 disabled 三态示例。
#[derive(Component)]
struct DisabledIndeterminateDemo;

/// 展示二态、disabled 与真实可交互三态 CheckBox。
pub(crate) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(CheckBoxDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start }
        Children [
            Text("CheckBox"),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                Text("Normal"),
                (@WidgetryCheckBox on(on_binary_change) Children [Text("Receive updates")]),
            ]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                Text("Disabled"),
                (@WidgetryCheckBox InteractionDisabled Children [Text("Unchecked")]),
                (@WidgetryCheckBox Checked InteractionDisabled Children [Text("Checked")]),
                (@WidgetryTriStateCheckbox template(|_| Ok(DisabledIndeterminateDemo)) InteractionDisabled Children [Text("Indeterminate")]),
            ]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                Text("Tri-State"),
                (@WidgetryTriStateCheckbox on(on_tri_state_change) Children [Text("Tri-State Checkbox")]),
            ]),
        ]
    }
}

/// 记录真实用户触发的二态变化。
fn on_binary_change(event: On<ValueChange<bool>>) {
    info!(entity = ?event.source, checked = event.value, "切换二态 CheckBox");
}

/// 记录真实用户触发的三态变化。
fn on_tri_state_change(event: On<ValueChange<WidgetryCheckState>>) {
    info!(entity = ?event.source, state = ?event.value, "切换三态 CheckBox");
}

/// ThemeChanged 时刷新页面分组标题的 foreground color。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<CheckBoxDemo>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

/// Scene 展开后设置示例初始 state，避免与 Widget 默认 state 重复挂载。
fn initialize_disabled_indeterminate(
    mut query: Query<&mut WidgetryCheckState, Added<DisabledIndeterminateDemo>>,
) {
    for mut state in &mut query {
        *state = WidgetryCheckState::Indeterminate;
    }
}

impl Plugin for CheckBoxDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(refresh_theme)
            .add_systems(Update, initialize_disabled_indeterminate);
    }
}
