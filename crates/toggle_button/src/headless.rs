use crate::{ToggleButtonError, ToggleButtonFailure, WidgetryToggleButtonChanged, error};
use bevy::prelude::*;
use bevy::ui::Checked;
use bevy::ui_widgets::{Checkbox, CheckboxPlugin, ValueChange};

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryToggleButtonHeadlessProps)]
#[require(Node, Checkbox, bevy::a11y::AccessibilityNode(accesskit::Node::new(accesskit::Role::Button)), bevy::picking::hover::Hovered, bevy_widgetry_core::pointer::WidgetryPointerPressed, bevy::input_focus::tab_navigation::TabIndex(-1))]
pub struct WidgetryToggleButtonHeadless;

#[derive(Default, Clone)]
pub struct WidgetryToggleButtonHeadlessProps {
    pub selected: bool,
}

impl WidgetryToggleButtonHeadless {
    fn scene(props: WidgetryToggleButtonHeadlessProps) -> impl Scene {
        bsn! {
            template(move |_| {
                let mut node = accesskit::Node::new(accesskit::Role::Button);
                node.set_toggled(if props.selected { accesskit::Toggled::True } else { accesskit::Toggled::False });
                Ok(bevy::a11y::AccessibilityNode(node))
            })
            @{props.selected.then_some(bsn! { Checked })}
        }
    }
}

pub struct WidgetryToggleButtonHeadlessPlugin;
impl Plugin for WidgetryToggleButtonHeadlessPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_core::ui::WidgetryUiPlugin>() {
            app.add_plugins(bevy_widgetry_core::ui::WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<CheckboxPlugin>() {
            app.add_plugins(CheckboxPlugin);
        }
        app.add_observer(commit_input);
        bevy_widgetry_internal_log::widgetry_debug!("WidgetryToggleButtonHeadlessPlugin 注册完成");
    }
}

impl WidgetryToggleButtonHeadless {
    pub fn selected(world: &World, entity: Entity) -> Result<bool, ToggleButtonFailure> {
        if world.get::<Self>(entity).is_none() {
            return Err(error::failure(
                ToggleButtonError::InvalidTarget { root: entity },
                "toggle_button.selected",
            ));
        }
        Ok(world.get::<Checked>(entity).is_some())
    }
    pub fn set_selected_in_world(
        world: &mut World,
        entity: Entity,
        selected: bool,
    ) -> Result<bool, ToggleButtonFailure> {
        if Self::selected(world, entity).map_err(|error| {
            error.with_context(
                bevy_widgetry_diagnostics::ErrorFrame::new("toggle_button.set_selected")
                    .field("root", entity),
            )
        })? == selected
        {
            return Ok(false);
        }
        if selected {
            world.entity_mut(entity).insert(Checked);
        } else {
            world.entity_mut(entity).remove::<Checked>();
        }
        world.trigger(WidgetryToggleButtonChanged { entity, selected });
        Ok(true)
    }
    pub fn set_selected(commands: &mut Commands, entity: Entity, selected: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_selected_in_world(world, entity, selected)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
}

fn commit_input(
    event: On<ValueChange<bool>>,
    query: Query<(), With<WidgetryToggleButtonHeadless>>,
    mut commands: Commands,
) {
    if query.contains(event.source) {
        WidgetryToggleButtonHeadless::set_selected(&mut commands, event.source, event.value);
    }
}
