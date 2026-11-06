use bevy_widgetry_theme::WidgetryTheme;

use crate::combo_box::WidgetryComboBox;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
use bevy::picking::pointer::PointerButton;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{
    Activate,
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
};
use bevy_widgetry_core::z_index;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListSelectionMode, WidgetryListSelectionState,
    WidgetryListView, WidgetryListViewItem, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxPopup;

pub(crate) fn scene<T: Send + Sync + 'static>(
    source: Entity,
    item_height: f32,
    renderer: WidgetryListViewRenderer<T>,
    search_content: Option<(
        crate::WidgetryComboBoxContent,
        crate::WidgetryComboBoxContent,
    )>,
    list_reference: bevy::ecs::template::EntityTemplate,
    search_reference: bevy::ecs::template::EntityTemplate,
) -> impl Scene {
    // Erase the composed ListView Scene type to bound stack usage when expanding nested Gallery Scenes.
    let list: Box<dyn Scene> = Box::new(bsn! {
        @crate::combo_box::root_reference(list_reference)
        @WidgetryListView::<T> { @source: source, @selection_mode: WidgetryListSelectionMode::Single, @item_height: item_height, @renderer: {renderer} }
        TabIndex(-1)
        Node { width: percent(100), height: percent(100), border: UiRect::ZERO }
    });
    let content: Box<dyn SceneList> = if let Some((search_prompt, no_results)) = search_content {
        Box::new(bsn_list! {
            @crate::combo_box::root_reference(search_reference)
            @bevy_widgetry_text_field::WidgetryTextField {
                @children: {Some(Box::new(bsn_list! {
                    bevy_widgetry_text_field::WidgetryTextFieldPrefix
                    Node { max_width: percent(50), margin: UiRect::right(px(8)) }
                    Children [{search_prompt.build()}]
                }) as Box<dyn SceneList>)},
            }
            Node { width: percent(100), height: px(40), flex_shrink: 0.0 }
            -- @{list} --
            crate::search::SearchEmpty
            Node { display: Display::None, min_height: px(item_height), flex_shrink: 0.0, align_items: AlignItems::Center }
            Children [{no_results.build()}]
        })
    } else {
        Box::new(bsn_list! { @{list} })
    };
    bsn! {
        ComboBoxPopup Visibility::Hidden GlobalZIndex({z_index::POPUP})
        Popover {
            positions: {vec![
                PopoverPlacement { side: PopoverSide::Bottom, align: PopoverAlign::Start, gap: 0.0 },
                PopoverPlacement { side: PopoverSide::Top, align: PopoverAlign::Start, gap: 0.0 },
            ]},
            window_margin: 8.0,
        }
        template(|context| Ok(BackgroundColor(context.resource::<WidgetryTheme>().combo_box.popup.normal.background)))
        template(|context| Ok(BorderColor::all(context.resource::<WidgetryTheme>().combo_box.popup.normal.border)))
        Node {
            position_type: PositionType::Absolute,
            width: percent(100), height: px(2), box_sizing: BoxSizing::BorderBox,
            flex_direction: FlexDirection::Column, align_items: AlignItems::Stretch,
            border: UiRect::all(px(1)), border_radius: BorderRadius::all(px(4)),
        }
        Children [{content}]
    }
}

pub(crate) fn handle_row_click<T: Send + Sync + 'static>(
    event: On<PointerClick>,
    roots: Query<(Entity, &crate::WidgetryComboBoxParts), With<crate::WidgetryComboBoxHeadless<T>>>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Primary {
        return;
    }
    let Some((root, parts)) = roots.iter().find(|(_, parts)| parts.list == event.entity) else {
        return;
    };
    let list = parts.list;
    let target = event.original_event_target();
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if !world
            .get::<crate::headless::OpenState>(root)
            .is_some_and(|state| state.0)
            || world.get::<InteractionDisabled>(list).is_some()
        {
            return Ok(());
        }
        let mut entity = target;
        let mut item = None;
        while entity != list {
            if world.get::<InteractionDisabled>(entity).is_some()
                || world.get::<WidgetryListViewState>(entity).is_some()
            {
                return Ok(());
            }
            if item.is_none() {
                item = world.get::<WidgetryListViewItem>(entity).copied();
            }
            let Some(parent) = world.get::<ChildOf>(entity) else {
                return Ok(());
            };
            entity = parent.parent();
        }
        let Some(item) = item else {
            return Ok(());
        };
        let valid = world
            .get::<WidgetryListView<T>>(list)
            .and_then(|view| world.get::<WidgetryListModel<T>>(view.source()))
            .is_some_and(|model| {
                model
                    .index_of(item.id)
                    .is_some_and(|index| model.is_disabled(index) == Some(false))
            })
            && world
                .get::<bevy_widgetry_list_view::WidgetryListProjection>(list)
                .is_some_and(|projection| projection.index_of(item.id).is_some());
        if valid
            && world
                .get::<WidgetryListSelectionState>(list)
                .is_some_and(|selection| selection.contains(item.id))
        {
            crate::headless::close::<T>(world, root)
                .map_err(crate::ComboFailure::into_bevy_error)?;
        }
        Ok(())
    });
}

pub(crate) fn handle_reselection<T: Send + Sync + 'static>(
    event: On<FocusedInput<KeyboardInput>>,
    roots: Query<(Entity, &crate::WidgetryComboBoxParts), With<crate::WidgetryComboBoxHeadless<T>>>,
    mut commands: Commands,
) {
    let list = event.focused_entity;
    if event.input.state != ButtonState::Pressed
        || !matches!(event.input.key_code, KeyCode::Enter | KeyCode::Space)
    {
        return;
    }
    let Some((root, _)) = roots.iter().find(|(_, parts)| parts.list == list) else {
        return;
    };
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if !world
            .get::<crate::headless::OpenState>(root)
            .is_some_and(|state| state.0)
            || world.get::<InteractionDisabled>(list).is_some()
            || world
                .get_resource::<InputFocus>()
                .is_none_or(|focus| focus.get() != Some(list))
        {
            return Ok(());
        }
        let active = world
            .get::<WidgetryListViewState>(list)
            .and_then(|state| state.active)
            .filter(|id| {
                world
                    .get::<WidgetryListSelectionState>(list)
                    .is_some_and(|selection| selection.contains(*id))
            });
        if let Some(active) = active {
            let valid = world
                .get::<WidgetryListView<T>>(list)
                .and_then(|view| world.get::<WidgetryListModel<T>>(view.source()))
                .is_some_and(|model| {
                    model
                        .index_of(active)
                        .is_some_and(|index| model.is_disabled(index) == Some(false))
                });
            if valid && !confirmation_disabled(world, list, active) {
                crate::headless::close::<T>(world, root)
                    .map_err(crate::ComboFailure::into_bevy_error)?;
            }
        }
        Ok(())
    });
}

pub(crate) fn confirmation_disabled(
    world: &World,
    list: Entity,
    active: WidgetryListItemId,
) -> bool {
    let Some(children) = world.get::<Children>(list) else {
        return false;
    };
    // Disabled ScrollAreas lose their wheel marker but retain their viewport identity.
    // Same-frame navigation may target an unmaterialized item, so check the viewport and content before the current row.
    for viewport in children
        .iter()
        .filter(|&child| world.get::<WidgetryScrollAreaViewport>(child).is_some())
    {
        if world.get::<InteractionDisabled>(viewport).is_some() {
            return true;
        }
        let Some(contents) = world.get::<Children>(viewport) else {
            continue;
        };
        for content in contents
            .iter()
            .filter(|&child| world.get::<WidgetryScrollAreaContent>(child).is_some())
        {
            if world.get::<InteractionDisabled>(content).is_some() {
                return true;
            }
            if world.get::<Children>(content).is_some_and(|rows| {
                rows.iter().any(|row| {
                    world
                        .get::<WidgetryListViewItem>(row)
                        .is_some_and(|item| item.id == active)
                        && world.get::<InteractionDisabled>(row).is_some()
                })
            }) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn handle_escape<T: Send + Sync + 'static>(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<
        (
            Entity,
            &crate::WidgetryComboBoxParts,
            &crate::headless::OpenState,
        ),
        With<crate::WidgetryComboBoxHeadless<T>>,
    >,
    focus: Res<InputFocus>,
    mut commands: Commands,
) {
    if event.input.key_code != KeyCode::Escape
        || event.input.state != ButtonState::Pressed
        || focus.get() != Some(event.focused_entity)
    {
        return;
    }
    let Some((root, parts, _)) = roots
        .iter()
        .find(|(_, parts, state)| parts.list == event.focused_entity && state.0)
    else {
        return;
    };
    let trigger = parts.trigger;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if crate::headless::close::<T>(world, root).map_err(crate::ComboFailure::into_bevy_error)?
            && world.get_entity(trigger).is_ok()
        {
            world
                .resource_mut::<InputFocus>()
                .set(trigger, FocusCause::Navigated);
        }
        Ok(())
    });
}

pub(crate) fn project_visibility<T: Send + Sync + 'static>(world: &mut World, root: Entity) {
    if world.get::<WidgetryComboBox<T>>(root).is_none() {
        return;
    }
    let Some(parts) = world.get::<crate::WidgetryComboBoxParts>(root).copied() else {
        return;
    };
    if crate::headless::owner(world, parts.popup) != Some(root) {
        return;
    }
    let open = world
        .get::<crate::headless::OpenState>(root)
        .is_some_and(|state| state.0);
    if let Some(mut visibility) = world.get_mut::<Visibility>(parts.popup) {
        visibility.set_if_neq(if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        });
    }
}

pub(crate) fn sync_visibility<T: Send + Sync + 'static>(
    roots: Query<
        (&crate::headless::OpenState, &crate::WidgetryComboBoxParts),
        With<WidgetryComboBox<T>>,
    >,
    mut popups: Query<&mut Visibility>,
) {
    for (state, parts) in &roots {
        if let Ok(mut visibility) = popups.get_mut(parts.popup) {
            visibility.set_if_neq(if state.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            });
        }
    }
}

pub(crate) fn sync_geometry<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query::<(
            Entity,
            &WidgetryComboBox<T>,
            &crate::WidgetryComboBoxHeadless<T>,
        )>()
        .iter(world)
        .map(|(root, combo, behavior)| {
            (
                root,
                behavior.source(),
                combo.item_height(),
                combo.max_visible_items(),
                world
                    .get::<crate::WidgetryComboBoxSearch<T>>(root)
                    .is_some(),
            )
        })
        .collect::<Vec<_>>();
    for (root, source, item_height, max_items, search) in roots {
        let Some(model) = world.get::<WidgetryListModel<T>>(source) else {
            continue;
        };
        let model_empty = model.is_empty();
        let Some(popup) = world.get::<Children>(root).and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<ComboBoxPopup>(child).is_some())
        }) else {
            continue;
        };
        let children = world
            .get::<Children>(popup)
            .map(|children| children.to_vec())
            .unwrap_or_default();
        let Some(list) = children
            .iter()
            .copied()
            .find(|child| world.get::<WidgetryListView<T>>(*child).is_some())
        else {
            continue;
        };
        let count = world
            .get::<bevy_widgetry_list_view::WidgetryListProjection>(list)
            .map_or(0, |projection| projection.len());
        let rows_height = count.min(max_items) as f32 * item_height;
        let empty = children
            .iter()
            .copied()
            .find(|child| world.get::<crate::search::SearchEmpty>(*child).is_some());
        let empty_height = empty
            .and_then(|empty| world.get::<ComputedNode>(empty))
            .map_or(item_height, |node| {
                node.size().y * node.inverse_scale_factor()
            })
            .max(item_height);
        if search {
            if let Some(mut node) = world.get_mut::<Node>(list) {
                let display = if count == 0 {
                    Display::None
                } else {
                    Display::Grid
                };
                if node.height != px(rows_height) {
                    node.height = px(rows_height);
                }
                if node.display != display {
                    node.display = display;
                }
            }
            if let Some(empty) = empty
                && let Some(mut node) = world.get_mut::<Node>(empty)
            {
                let display = if count == 0 {
                    Display::Flex
                } else {
                    Display::None
                };
                if node.display != display {
                    node.display = display;
                }
            }
        }
        let height = rows_height
            + if search {
                40.0 + if count == 0 { empty_height } else { 0.0 }
            } else {
                0.0
            }
            + 2.0;
        if let Some(mut node) = world.get_mut::<Node>(popup)
            && node.height != px(height)
        {
            node.height = px(height);
        }
        if !search
            && model_empty
            && let Err(error) = crate::headless::close::<T>(world, root)
        {
            world
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> {
                    Err(error.into_bevy_error())
                });
        }
    }
}

pub(crate) fn handle_field_activate<T: Send + Sync + 'static>(
    event: On<Activate>,
    roots: Query<(Entity, &crate::WidgetryComboBoxParts), With<crate::WidgetryComboBoxHeadless<T>>>,
    mut commands: Commands,
) {
    let Some((root, _)) = roots
        .iter()
        .find(|(_, parts)| parts.trigger == event.entity)
    else {
        return;
    };
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if crate::WidgetryComboBoxHeadless::<T>::is_open(world, root)
            .map_err(crate::ComboFailure::into_bevy_error)?
        {
            crate::WidgetryComboBoxHeadless::<T>::close_in_world(world, root)
        } else {
            crate::WidgetryComboBoxHeadless::<T>::open_in_world(world, root)
        }
        .map(|_| ())
        .map_err(crate::ComboFailure::into_bevy_error)
    });
}

pub(crate) fn handle_outside_click<T: Send + Sync + 'static>(
    event: On<PointerClick>,
    roots: Query<(Entity, &crate::headless::OpenState), With<crate::WidgetryComboBoxHeadless<T>>>,
    mut commands: Commands,
) {
    let target = event.original_event_target();
    for (root, state) in &roots {
        if state.0 {
            commands.queue(move |world: &mut World| -> Result<(), BevyError> {
                if !crate::headless::contains(world, root, target) {
                    crate::headless::close::<T>(world, root)
                        .map_err(crate::ComboFailure::into_bevy_error)?;
                }
                Ok(())
            });
        }
    }
}
