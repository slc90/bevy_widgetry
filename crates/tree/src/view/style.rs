use super::{TreeExpander, WidgetryTreeView};
use crate::{WidgetryTreeModel, WidgetryTreeVisibleItem};
use bevy::ecs::entity::EntityHashSet;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::{color::WidgetryStyleOwner, foreground::ResolvedForeground};
use bevy_widgetry_internal_log::widgetry_error;
use bevy_widgetry_list_view::WidgetryListView;
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode, WidgetryTreeColors};

#[derive(Component, Default)]
pub(crate) struct TreeStyle(Option<WidgetryTreeColors>);

#[derive(Resource, Default)]
pub(crate) struct ColorChanges {
    palettes: EntityHashSet,
    lists: EntityHashSet,
    buttons: EntityHashSet,
}

#[cfg(test)]
thread_local! {
    pub(super) static COLOR_WORK: std::cell::RefCell<(Vec<Entity>, Vec<Entity>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<ColorChanges>()
        .add_observer(removed_inputs)
        .add_observer(refresh_colors);
}

fn removed_inputs(
    event: On<
        Remove<(
            Hovered,
            Pressed,
            InteractionDisabled,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
            crate::colors::ColorState,
            TreeStyle,
        )>,
    >,
    buttons: Query<(), With<TreeExpander>>,
    roots: Query<(), With<WidgetryTreeView>>,
    mut changes: ResMut<ColorChanges>,
) {
    if buttons.contains(event.entity) {
        changes.buttons.insert(event.entity);
    }
    if roots.contains(event.entity) {
        changes.palettes.insert(event.entity);
    }
}

pub(crate) fn collect_changes(
    roots: Query<
        Entity,
        (
            With<WidgetryTreeView>,
            Or<(
                Added<WidgetryTreeView>,
                Added<TreeStyle>,
                Changed<crate::colors::ColorState>,
            )>,
        ),
    >,
    lists: Query<
        Entity,
        (
            With<WidgetryListView<WidgetryTreeVisibleItem>>,
            Or<(
                Added<WidgetryListView<WidgetryTreeVisibleItem>>,
                Changed<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>,
            )>,
        ),
    >,
    buttons: Query<
        Entity,
        (
            With<TreeExpander>,
            Or<(
                Changed<TreeExpander>,
                Changed<WidgetryStyleOwner<WidgetryButton>>,
                Changed<Hovered>,
                Added<Pressed>,
                Added<InteractionDisabled>,
                Added<BackgroundColor>,
                Added<BorderColor>,
                Added<ResolvedForeground>,
            )>,
        ),
    >,
    mut changes: ResMut<ColorChanges>,
) {
    changes.palettes.extend(roots.iter());
    changes.lists.extend(lists.iter());
    changes.buttons.extend(buttons.iter());
}

fn palette(world: &World, root: Entity) -> Result<WidgetryTreeColors, BevyError> {
    world
        .get::<TreeStyle>(root)
        .and_then(|style| style.0)
        .ok_or_else(|| {
            widgetry_error!(?root, "TreeView 缺失已解析配色");
            BevyError::error("TreeView 缺失已解析配色")
        })
}

pub(crate) fn update_colors(world: &mut World) -> Result<(), BevyError> {
    let mut changes = std::mem::take(&mut *world.resource_mut::<ColorChanges>());
    let mut failure = None;
    let mut resolved = EntityHashSet::default();
    for root in changes.palettes.drain().collect::<Vec<_>>() {
        if world.get::<WidgetryTreeView>(root).is_none() {
            continue;
        }
        let result = (|| {
            #[cfg(test)]
            COLOR_WORK.with_borrow_mut(|work| work.0.push(root));
            let theme = world.resource::<WidgetryThemeMode>().colors().tree;
            let colors = crate::WidgetryTreeColorOverrides::get(world, root)?.resolve(&theme);
            let Some(mut style) = world.get_mut::<TreeStyle>(root) else {
                widgetry_error!(?root, "TreeView 缺失配色缓存");
                return Err(BevyError::error("TreeView 缺失配色缓存"));
            };
            style.0 = Some(colors);
            resolved.insert(root);
            Ok(())
        })();
        if let Err(error) = result {
            changes.palettes.insert(root);
            failure.get_or_insert(error);
        }
    }
    if !resolved.is_empty() {
        for (list, owner) in world
            .query::<(
                Entity,
                &WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>,
            )>()
            .iter(world)
        {
            if resolved.contains(&owner.entity) {
                changes.lists.insert(list);
            }
        }
        for (button, owner) in world
            .query_filtered::<(Entity, &WidgetryStyleOwner<WidgetryButton>), With<TreeExpander>>()
            .iter(world)
        {
            if resolved.contains(&owner.entity) {
                changes.buttons.insert(button);
            }
        }
    }
    for list in changes.lists.drain().collect::<Vec<_>>() {
        if world
            .get::<WidgetryListView<WidgetryTreeVisibleItem>>(list)
            .is_none()
        {
            continue;
        }
        let Some(root) = world
            .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>(list)
            .map(|owner| owner.entity)
        else {
            continue;
        };
        if changes.palettes.contains(&root) {
            changes.lists.insert(list);
            continue;
        }
        let result = palette(world, root).and_then(|colors| {
            bevy_widgetry_list_view::internal::apply_owned_list_colors::<WidgetryTreeVisibleItem>(
                world,
                list,
                &bevy_widgetry_theme::WidgetryListViewColors {
                    container: colors.container,
                    item: colors.item,
                },
            )
        });
        if let Err(error) = result {
            changes.lists.insert(list);
            failure.get_or_insert(error);
        }
    }
    for button in changes.buttons.drain().collect::<Vec<_>>() {
        let Some(expander) = world.get::<TreeExpander>(button).copied() else {
            continue;
        };
        let Some(root) = world
            .get::<WidgetryStyleOwner<WidgetryButton>>(button)
            .map(|owner| owner.entity)
        else {
            continue;
        };
        if changes.palettes.contains(&root) {
            changes.buttons.insert(button);
            continue;
        }
        let result = palette(world, root).and_then(|colors| {
            #[cfg(test)]
            COLOR_WORK.with_borrow_mut(|work| work.1.push(button));
            let expanded = world
                .get::<WidgetryTreeModel>(expander.source)
                .is_some_and(|model| model.state().is_expanded(expander.node));
            let colors = if expanded {
                colors.expander.expanded
            } else {
                colors.expander.collapsed
            };
            let state = if world.get::<InteractionDisabled>(button).is_some() {
                colors.disabled
            } else if world.get::<Pressed>(button).is_some() {
                colors.pressed
            } else if world
                .get::<Hovered>(button)
                .is_some_and(|hovered| hovered.0)
            {
                colors.hovered
            } else {
                colors.normal
            };
            bevy_widgetry_button::internal::apply_owned_button_colors(
                world,
                button,
                bevy_widgetry_theme::WidgetryButtonStateColors {
                    background: state.background,
                    border: state.border,
                    foreground: state.foreground,
                },
            )
        });
        if let Err(error) = result {
            changes.buttons.insert(button);
            failure.get_or_insert(error);
        }
    }
    let mut pending = world.resource_mut::<ColorChanges>();
    pending.palettes.extend(changes.palettes);
    pending.lists.extend(changes.lists);
    pending.buttons.extend(changes.buttons);
    failure.map_or(Ok(()), Err)
}

fn refresh_colors(
    _event: On<WidgetryThemeChanged>,
    roots: Query<Entity, With<WidgetryTreeView>>,
    mut changes: ResMut<ColorChanges>,
    mut commands: Commands,
) {
    changes.palettes.extend(roots.iter());
    commands.queue(update_colors);
}
