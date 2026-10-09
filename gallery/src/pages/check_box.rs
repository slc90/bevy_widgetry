use bevy::prelude::*;
use bevy::ui::{Checked, InteractionDisabled};
use bevy::ui_widgets::ValueChange;
use bevy_widgetry::check_box::{WidgetryCheckBox, WidgetryCheckState, WidgetryTriStateCheckbox};

pub(crate) struct CheckBoxDemoPlugin;

#[derive(Component)]
struct CheckBoxDemo;

#[derive(Component)]
struct DisabledIndeterminateDemo;

pub(crate) fn scene() -> impl Scene {
    bsn! {
        template(|_| Ok(CheckBoxDemo))
        Node { flex_direction: FlexDirection::Column, row_gap: px(16), align_items: AlignItems::Start }
        Children [
            Text("CheckBox") bevy_widgetry::text::WidgetryText,
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                Text("Normal") bevy_widgetry::text::WidgetryText,
                (@WidgetryCheckBox on(on_binary_change) Children [Text("Receive updates") bevy_widgetry::text::WidgetryText]),
            ]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                Text("Disabled") bevy_widgetry::text::WidgetryText,
                (@WidgetryCheckBox InteractionDisabled Children [Text("Unchecked") bevy_widgetry::text::WidgetryText]),
                (@WidgetryCheckBox Checked InteractionDisabled Children [Text("Checked") bevy_widgetry::text::WidgetryText]),
                (@WidgetryTriStateCheckbox template(|_| Ok(DisabledIndeterminateDemo)) InteractionDisabled Children [Text("Indeterminate") bevy_widgetry::text::WidgetryText]),
            ]),
            (Node { flex_direction: FlexDirection::Column, row_gap: px(8) } Children [
                Text("Tri-State") bevy_widgetry::text::WidgetryText,
                (@WidgetryTriStateCheckbox on(on_tri_state_change) Children [Text("Tri-State Checkbox") bevy_widgetry::text::WidgetryText]),
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
        app.add_systems(Update, initialize_disabled_indeterminate);
    }
}
