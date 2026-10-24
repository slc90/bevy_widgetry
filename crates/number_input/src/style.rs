use crate::{
    editing::{Parts, ReadOnly, StepButton},
    value::invalid,
};
use bevy::{
    ecs::entity::EntityHashSet,
    input_focus::InputFocus,
    picking::hover::Hovered,
    prelude::*,
    ui::{InteractionDisabled, Pressed},
};
use bevy_widgetry_core::{
    color::WidgetryStyleOwner, foreground::ResolvedForeground, input_target::WidgetryInputTarget,
    ui::WidgetryUiSystems,
};
use bevy_widgetry_text_field::WidgetryTextField;
use bevy_widgetry_theme::WidgetryThemeMode;

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        establish_owners.in_set(WidgetryUiSystems::StyleOwners),
    )
    .add_systems(PostUpdate, update.in_set(WidgetryUiSystems::Colors));
}

fn establish_owners(roots: Query<(Entity, &Parts), Added<Parts>>, mut commands: Commands) {
    for (root, parts) in &roots {
        commands
            .entity(parts.field)
            .insert(WidgetryStyleOwner::<WidgetryTextField>::new(root));
    }
}

fn update(
    mode: Res<WidgetryThemeMode>,
    focus: Res<InputFocus>,
    roots: Query<Entity, With<Parts>>,
    changed: Query<Entity, (With<Parts>, Or<(Added<Parts>, Added<InteractionDisabled>)>)>,
    buttons: Query<&StepButton>,
    changed_buttons: Query<
        &StepButton,
        Or<(Changed<Hovered>, Added<Pressed>, Added<InteractionDisabled>)>,
    >,
    fields: Query<&WidgetryStyleOwner<WidgetryTextField>>,
    changed_fields: Query<&WidgetryStyleOwner<WidgetryTextField>, Changed<Hovered>>,
    mut removed_disabled: RemovedComponents<InteractionDisabled>,
    mut removed_pressed: RemovedComponents<Pressed>,
    mut removed_hovered: RemovedComponents<Hovered>,
    mut commands: Commands,
) {
    let mut dirty: EntityHashSet = if mode.is_changed() || focus.is_changed() {
        roots.iter().collect()
    } else {
        changed.iter().collect()
    };
    dirty.extend(changed_buttons.iter().map(|button| button.root));
    dirty.extend(changed_fields.iter().map(|owner| owner.entity));
    for entity in removed_disabled
        .read()
        .chain(removed_pressed.read())
        .chain(removed_hovered.read())
    {
        if roots.contains(entity) {
            dirty.insert(entity);
        }
        if let Ok(button) = buttons.get(entity) {
            dirty.insert(button.root);
        }
        if let Ok(owner) = fields.get(entity) {
            dirty.insert(owner.entity);
        }
    }
    for root in dirty {
        commands.queue(move |world: &mut World| {
            if world.get::<Parts>(root).is_none() {
                return Ok(());
            }
            apply(world, root)
        });
    }
}

fn apply(world: &mut World, root: Entity) -> Result {
    let theme = world.resource::<WidgetryThemeMode>().colors();
    let parts = world
        .get::<Parts>(root)
        .ok_or_else(|| invalid("NumberInput style parts are missing"))?;
    let (field, decrease, increase) = (parts.field, parts.decrease, parts.increase);
    let palette = if world.get::<ReadOnly>(root).is_some() {
        theme.text_field.read_only
    } else {
        theme.text_field.editable
    };
    let editor = world
        .get::<WidgetryInputTarget>(field)
        .ok_or_else(|| invalid("NumberInput style editor is missing"))?
        .target;
    let state = if world.get::<InteractionDisabled>(root).is_some() {
        palette.disabled
    } else if world.resource::<InputFocus>().get() == Some(editor) {
        palette.focused
    } else if world.get::<Hovered>(field).is_some_and(|h| h.0) {
        palette.hovered
    } else {
        palette.normal
    };
    world
        .get_mut::<BackgroundColor>(root)
        .ok_or_else(|| invalid("NumberInput shell background is missing"))?
        .set_if_neq(BackgroundColor(state.background));
    world
        .get_mut::<BorderColor>(root)
        .ok_or_else(|| invalid("NumberInput shell border is missing"))?
        .set_if_neq(BorderColor::all(state.border));
    bevy_widgetry_text_field::internal::apply_owned_text_field_colors(
        world,
        field,
        &theme.text_field,
    )?;
    for button in [decrease, increase] {
        let colors = if world.get::<InteractionDisabled>(button).is_some() {
            theme.button.disabled
        } else if world.get::<Pressed>(button).is_some() {
            theme.button.pressed
        } else if world.get::<Hovered>(button).is_some_and(|h| h.0) {
            theme.button.hovered
        } else {
            theme.button.normal
        };
        world
            .get_mut::<BackgroundColor>(button)
            .ok_or_else(|| invalid("NumberInput step background is missing"))?
            .set_if_neq(BackgroundColor(colors.background));
        world
            .get_mut::<BorderColor>(button)
            .ok_or_else(|| invalid("NumberInput separator is missing"))?
            .set_if_neq(BorderColor::all(state.border));
        world
            .get_mut::<ResolvedForeground>(button)
            .ok_or_else(|| invalid("NumberInput step foreground is missing"))?
            .set_if_neq(ResolvedForeground(colors.foreground));
    }
    Ok(())
}
