use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::check_box::{WidgetryCheckBox, WidgetryCheckState, WidgetryTriStateCheckbox};
use bevy_widgetry::style::{ForegroundColor, ThemeChanged, ThemeMode};

pub(crate) struct CheckBoxDemoPlugin;

#[derive(Component)]
struct CheckBoxDemo;

#[derive(Component)]
struct DisabledIndeterminateDemo;

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

fn on_binary_change(event: On<ValueChange<bool>>) {
    info!(entity = ?event.source, checked = event.value, "切换二态 CheckBox");
}

fn on_tri_state_change(event: On<ValueChange<WidgetryCheckState>>) {
    info!(entity = ?event.source, state = ?event.value, "切换三态 CheckBox");
}

fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<CheckBoxDemo>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

fn initialize_disabled_indeterminate(
    query: Query<Entity, Added<DisabledIndeterminateDemo>>,
    mut commands: Commands,
) {
    for entity in &query {
        WidgetryTriStateCheckbox::set_state(
            &mut commands,
            entity,
            WidgetryCheckState::Indeterminate,
        );
    }
}

impl Plugin for CheckBoxDemoPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(refresh_theme)
            .add_systems(Update, initialize_disabled_indeterminate);
    }
}
