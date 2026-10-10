use crate::{
    WidgetryComboBox,
    field::{ComboBoxDropdownIcon, ComboBoxField},
    popup::ComboBoxPopup,
};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::{color::WidgetryStyleOwner, foreground::ResolvedForeground};
use bevy_widgetry_list_view::WidgetryListView;
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

pub(crate) fn establish_owners<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query_filtered::<(Entity, &Children), With<WidgetryComboBox<T>>>()
        .iter(world)
        .map(|(r, c)| (r, c.to_vec()))
        .collect::<Vec<_>>();
    for (root, children) in roots {
        for child in children {
            if world.get::<ComboBoxField>(child).is_some()
                && world
                    .get::<WidgetryStyleOwner<WidgetryButton>>(child)
                    .is_none()
            {
                world
                    .entity_mut(child)
                    .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
            }
            if world.get::<ComboBoxPopup>(child).is_some() {
                let lists = world
                    .get::<Children>(child)
                    .map(|c| c.to_vec())
                    .unwrap_or_default();
                for list in lists {
                    if world.get::<WidgetryListView<T>>(list).is_some()
                        && world
                            .get::<WidgetryStyleOwner<WidgetryListView<T>>>(list)
                            .is_none()
                    {
                        world
                            .entity_mut(list)
                            .insert(WidgetryStyleOwner::<WidgetryListView<T>>::new(root));
                    }
                }
            }
        }
    }
}
fn failure(entity: Entity) -> BevyError {
    bevy_widgetry_log::widgetry_error!(?entity, "ComboBox 缺失颜色部件");
    BevyError::error("ComboBox 缺失颜色部件")
}
pub fn apply_owned_combo_colors<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryComboBoxColors,
) -> Result<(), BevyError> {
    let children = world
        .get::<Children>(root)
        .ok_or_else(|| failure(root))?
        .to_vec();
    let field = children
        .iter()
        .copied()
        .find(|e| world.get::<ComboBoxField>(*e).is_some())
        .ok_or_else(|| failure(root))?;
    let popup = children
        .iter()
        .copied()
        .find(|e| world.get::<ComboBoxPopup>(*e).is_some())
        .ok_or_else(|| failure(root))?;
    let disabled = world.get::<InteractionDisabled>(field).is_some();
    let open = world.get::<Visibility>(popup) == Some(&Visibility::Visible);
    let state = if disabled {
        colors.field.disabled
    } else if open {
        colors.field.open
    } else if world.get::<Pressed>(field).is_some() {
        colors.field.pressed
    } else if world.get::<Hovered>(field).is_some_and(|h| h.0) {
        colors.field.hovered
    } else {
        colors.field.normal
    };
    bevy_widgetry_button::internal::apply_owned_button_colors(
        world,
        field,
        bevy_widgetry_theme::WidgetryButtonStateColors {
            background: state.background,
            border: state.border,
            foreground: state.foreground,
        },
    )?;
    let icons = world
        .get::<Children>(field)
        .ok_or_else(|| failure(field))?
        .to_vec();
    for icon in icons {
        if world.get::<ComboBoxDropdownIcon>(icon).is_some()
            && world.get::<ResolvedForeground>(icon)
                != Some(&ResolvedForeground(state.indicator_foreground))
        {
            world
                .entity_mut(icon)
                .insert(ResolvedForeground(state.indicator_foreground));
        }
    }
    let popup_state = if world.get::<InteractionDisabled>(popup).is_some() {
        colors.popup.disabled
    } else {
        colors.popup.normal
    };
    world
        .get_mut::<BackgroundColor>(popup)
        .ok_or_else(|| failure(popup))?
        .set_if_neq(BackgroundColor(popup_state.background));
    world
        .get_mut::<BorderColor>(popup)
        .ok_or_else(|| failure(popup))?
        .set_if_neq(BorderColor::all(popup_state.border));
    if world.get::<ResolvedForeground>(popup) != Some(&ResolvedForeground(popup_state.foreground)) {
        world
            .entity_mut(popup)
            .insert(ResolvedForeground(popup_state.foreground));
    }
    let lists = world
        .get::<Children>(popup)
        .ok_or_else(|| failure(popup))?
        .to_vec();
    let list = lists
        .into_iter()
        .find(|e| world.get::<WidgetryListView<T>>(*e).is_some())
        .ok_or_else(|| failure(popup))?;
    bevy_widgetry_list_view::internal::apply_owned_list_colors::<T>(world, list, &colors.list)
}
pub(crate) fn update<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let theme = world.resource::<WidgetryThemeMode>().colors().combo_box;
    let roots = world
        .query_filtered::<Entity, (
            With<WidgetryComboBox<T>>,
            Without<WidgetryStyleOwner<WidgetryComboBox<T>>>,
        )>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let colors = world
            .get::<crate::colors::ColorState>(root)
            .ok_or_else(|| failure(root))?
            .0
            .resolve(&theme);
        apply_owned_combo_colors::<T>(world, root, &colors)?;
    }
    Ok(())
}
pub(crate) fn refresh_theme<T: Send + Sync + 'static>(
    _event: On<WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(update::<T>);
}

pub(crate) fn own_field<T: Send + Sync + 'static>(
    event: On<Add<ComboBoxField>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    if let Ok(parent) = parents.get(event.entity)
        && roots.contains(parent.parent())
    {
        commands
            .entity(event.entity)
            .insert(WidgetryStyleOwner::<WidgetryButton>::new(parent.parent()));
    }
}
pub(crate) fn own_list<T: Send + Sync + 'static>(
    event: On<Add<WidgetryListView<T>>>,
    parents: Query<&ChildOf>,
    popups: Query<(), With<ComboBoxPopup>>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    if let Ok(parent) = parents.get(event.entity)
        && popups.contains(parent.parent())
        && let Ok(root) = parents.get(parent.parent())
        && roots.contains(root.parent())
    {
        commands
            .entity(event.entity)
            .insert(WidgetryStyleOwner::<WidgetryListView<T>>::new(
                root.parent(),
            ));
    }
}
