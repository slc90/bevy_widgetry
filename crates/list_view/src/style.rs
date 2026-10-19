use crate::WidgetryListKey;
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

use crate::virtualization::ListRuntime;
use crate::{WidgetryListView, WidgetryListViewItem, WidgetryListViewState};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed, Selected};
use bevy_widgetry_core::foreground::ResolvedForeground;

#[cfg(test)]
#[derive(Resource, Default)]
pub(crate) struct StyleWork {
    roots: usize,
    rows: usize,
}

#[derive(Debug, PartialEq)]
struct RowStyle {
    background: Color,
    border: Color,
    foreground: Color,
}

fn root_border(
    colors: &bevy_widgetry_theme::WidgetryListViewColors,
    disabled: bool,
    focused: bool,
) -> Color {
    if disabled {
        colors.container.disabled.border
    } else if focused {
        colors.container.focused.border
    } else {
        colors.container.normal.border
    }
}

fn resolve_row(
    colors: &bevy_widgetry_theme::WidgetryListViewColors,
    disabled: bool,
    pressed: bool,
    hovered: bool,
    selected: bool,
    active: bool,
) -> RowStyle {
    let colors = &colors.item;
    let state = if disabled {
        colors.disabled
    } else if pressed {
        colors.pressed
    } else if hovered {
        colors.hovered
    } else if selected {
        colors.selected
    } else {
        colors.normal
    };
    RowStyle {
        background: state.background,
        border: if active {
            if disabled {
                colors.disabled_active_border
            } else {
                colors.active_border
            }
        } else {
            state.border
        },
        foreground: state.foreground,
    }
}

#[derive(Component)]
pub(crate) struct RowOwner(pub(crate) Entity);

#[derive(Clone, Copy, PartialEq)]
struct AppliedStyle {
    colors: bevy_widgetry_theme::WidgetryListViewColors,
    disabled: bool,
    focused: bool,
}

#[derive(Component)]
pub(crate) struct StyleInvalidation {
    full: bool,
    rows: EntityHashSet,
    applied: Option<AppliedStyle>,
}

impl Default for StyleInvalidation {
    fn default() -> Self {
        Self {
            full: true,
            rows: default(),
            applied: None,
        }
    }
}

pub(crate) fn install_typed<K: WidgetryListKey>(app: &mut App) {
    app.add_observer(
        |event: On<Insert<crate::WidgetryListSelectionState<K>>>,
         rows: Query<&RowOwner>,
         mut roots: Query<&mut StyleInvalidation>| {
            invalidate(event.entity, &rows, &mut roots)
        },
    );
    app.add_observer(
        |event: On<Remove<crate::WidgetryListSelectionState<K>>>,
         rows: Query<&RowOwner>,
         mut roots: Query<&mut StyleInvalidation>| {
            invalidate(event.entity, &rows, &mut roots)
        },
    );
    app.add_observer(
        |event: On<Remove<WidgetryListViewState<K>>>,
         rows: Query<&RowOwner>,
         mut roots: Query<&mut StyleInvalidation>| {
            invalidate(event.entity, &rows, &mut roots)
        },
    );
    app.add_observer(
        |event: On<Insert<WidgetryListViewState<K>>>,
         rows: Query<&RowOwner>,
         mut roots: Query<&mut StyleInvalidation>| {
            invalidate(event.entity, &rows, &mut roots)
        },
    );
    app.add_observer(
        |event: On<Insert<WidgetryListViewItem<K>>>,
         rows: Query<&RowOwner>,
         mut roots: Query<&mut StyleInvalidation>| {
            invalidate(event.entity, &rows, &mut roots)
        },
    );
}

pub(crate) fn install(app: &mut App) {
    app.add_observer(on_insert).add_observer(on_remove);
}

fn invalidate(entity: Entity, rows: &Query<&RowOwner>, roots: &mut Query<&mut StyleInvalidation>) {
    if let Ok(mut style) = roots.get_mut(entity) {
        style.full = true;
    } else if let Ok(owner) = rows.get(entity)
        && let Ok(mut style) = roots.get_mut(owner.0)
    {
        style.rows.insert(entity);
    }
}

fn on_insert(
    event: On<
        Insert<(
            Hovered,
            Pressed,
            Selected,
            InteractionDisabled,
            Children,
            crate::drag::DropHint,
        )>,
    >,
    rows: Query<&RowOwner>,
    mut roots: Query<&mut StyleInvalidation>,
) {
    invalidate(event.entity, &rows, &mut roots);
}

fn on_remove(
    event: On<
        Remove<(
            Hovered,
            Pressed,
            Selected,
            InteractionDisabled,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
            Children,
            crate::drag::DropHint,
        )>,
    >,
    rows: Query<&RowOwner>,
    mut roots: Query<&mut StyleInvalidation>,
) {
    invalidate(event.entity, &rows, &mut roots);
}

pub(crate) fn invalidate_row(world: &mut World, root: Entity, row: Entity) {
    if let Some(mut style) = world.get_mut::<StyleInvalidation>(root) {
        style.rows.insert(row);
    }
}

pub(crate) fn collect_changes<T: Send + Sync + 'static, K: WidgetryListKey>(
    colors: Query<
        Entity,
        (
            With<WidgetryListView<T, K>>,
            Changed<crate::colors::ColorState>,
        ),
    >,
    mut styles: Query<&mut StyleInvalidation, With<WidgetryListView<T, K>>>,
    focus: Res<InputFocus>,
    navigation_owners: Query<
        (Entity, &crate::behavior::NavigationFocusOwner),
        With<WidgetryListView<T, K>>,
    >,
    mut previous: Local<Option<Entity>>,
    mut removed_owners: RemovedComponents<
        bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryListView<T, K>>,
    >,
) {
    for entity in colors.iter().chain(removed_owners.read()) {
        if let Ok(mut style) = styles.get_mut(entity) {
            style.full = true;
        }
    }
    if *previous != focus.get() {
        for entity in [*previous, focus.get()].into_iter().flatten() {
            if let Ok(mut style) = styles.get_mut(entity) {
                style.full = true;
            }
            for (root, owner) in &navigation_owners {
                if owner.0 == entity
                    && let Ok(mut style) = styles.get_mut(root)
                {
                    style.full = true;
                }
            }
        }
        *previous = focus.get();
    }
}

pub fn apply_owned_list_colors_with_key<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryListViewColors,
) -> Result<(), BevyError> {
    world
        .get::<WidgetryListView<T, K>>(root)
        .ok_or_else(|| invalid_structure(root))?;
    let state = *world
        .get::<WidgetryListViewState<K>>(root)
        .ok_or_else(|| invalid_structure(root))?;
    let input = AppliedStyle {
        colors: *colors,
        disabled: world.get::<InteractionDisabled>(root).is_some(),
        focused: crate::behavior::is_focused(world, root),
    };
    let (full, pending) = {
        let mut style = world
            .get_mut::<StyleInvalidation>(root)
            .ok_or_else(|| invalid_structure(root))?;
        let full = style.full || style.applied != Some(input);
        if !full && style.rows.is_empty() {
            return Ok(());
        }
        (full, std::mem::take(&mut style.rows))
    };
    let result = (|| -> Result<(), BevyError> {
        if full {
            #[cfg(test)]
            if let Some(mut work) = world.get_resource_mut::<StyleWork>() {
                work.roots += 1;
            }
            let container = if input.disabled {
                colors.container.disabled
            } else if input.focused {
                colors.container.focused
            } else {
                colors.container.normal
            };
            world
                .get_mut::<BackgroundColor>(root)
                .ok_or_else(|| invalid_structure(root))?
                .set_if_neq(BackgroundColor(container.background));
            world
                .get_mut::<BorderColor>(root)
                .ok_or_else(|| invalid_structure(root))?
                .set_if_neq(BorderColor::all(root_border(
                    colors,
                    input.disabled,
                    input.focused,
                )));
            world
                .get_mut::<ResolvedForeground>(root)
                .ok_or_else(|| invalid_structure(root))?
                .set_if_neq(ResolvedForeground(container.foreground));
        }
        let rows = if full {
            world
                .get::<ListRuntime>(root)
                .map(|runtime| runtime.rows.clone())
                .unwrap_or_default()
        } else {
            pending
                .into_iter()
                .filter(|row| {
                    world
                        .get::<RowOwner>(*row)
                        .is_some_and(|owner| owner.0 == root)
                })
                .collect()
        };
        for row in rows {
            if !world.entities().contains(row) {
                if full {
                    return Err(invalid_structure(row));
                }
                continue;
            }
            let item = *world
                .get::<WidgetryListViewItem<K>>(row)
                .ok_or_else(|| invalid_structure(row))?;
            #[cfg(test)]
            if let Some(mut work) = world.get_resource_mut::<StyleWork>() {
                work.rows += 1;
            }
            let style = resolve_row(
                colors,
                input.disabled || world.get::<InteractionDisabled>(row).is_some(),
                world.get::<Pressed>(row).is_some(),
                world.get::<Hovered>(row).is_some_and(|h| h.0),
                world.get::<Selected>(row).is_some(),
                input.focused && state.active == Some(item.id),
            );
            world
                .get_mut::<BackgroundColor>(row)
                .ok_or_else(|| invalid_structure(row))?
                .set_if_neq(BackgroundColor(style.background));
            let mut border = BorderColor::all(style.border);
            if let Some(hint) = world.get::<crate::drag::DropHint>(row) {
                if !hint.allowed {
                    border = BorderColor::all(colors.item.disabled.foreground);
                } else {
                    match hint.placement {
                        crate::WidgetryListDropPlacement::Before => {
                            border.top = colors.item.active_border
                        }
                        crate::WidgetryListDropPlacement::After => {
                            border.bottom = colors.item.active_border
                        }
                        crate::WidgetryListDropPlacement::Inside => {
                            border = BorderColor::all(colors.item.active_border)
                        }
                    }
                }
            }
            world
                .get_mut::<BorderColor>(row)
                .ok_or_else(|| invalid_structure(row))?
                .set_if_neq(border);
            world
                .get_mut::<ResolvedForeground>(row)
                .ok_or_else(|| invalid_structure(row))?
                .set_if_neq(ResolvedForeground(style.foreground));
        }
        Ok(())
    })();
    if let Some(mut style) = world.get_mut::<StyleInvalidation>(root) {
        style.full = result.is_err();
        if result.is_ok() {
            style.applied = Some(input);
        }
    }
    result
}

fn invalid_structure(entity: Entity) -> BevyError {
    bevy_widgetry_internal_log::widgetry_error!(?entity, "ListView 缺失颜色主体或必需输出");
    BevyError::error("ListView 缺失颜色主体或必需输出")
}

pub(crate) fn update<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
) -> Result<(), BevyError> {
    let theme = world.resource::<WidgetryThemeMode>().colors().list_view;
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        let result = (|| -> Result<(), BevyError> {
            let style = world
                .get::<StyleInvalidation>(root)
                .ok_or_else(|| invalid_structure(root))?;
            if !style.full && style.rows.is_empty() {
                return Ok(());
            }
            let owned = world
                .get::<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryListView<T, K>>>(root)
                .is_some();
            let colors = if owned {
                // Local input changes reuse the last palette supplied by the actual composition owner.
                let Some(applied) = style.applied else {
                    return Ok(());
                };
                applied.colors
            } else if !style.full
                && let Some(applied) = style.applied
            {
                applied.colors
            } else {
                world
                    .get::<crate::colors::ColorState>(root)
                    .ok_or_else(|| invalid_structure(root))?
                    .0
                    .resolve(&theme)
            };
            apply_owned_list_colors_with_key::<T, K>(world, root, &colors)
        })();
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn refresh_theme<T: Send + Sync + 'static, K: WidgetryListKey>(
    _event: On<WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(|world: &mut World| -> Result<(), BevyError> {
        let roots = world
            .query_filtered::<Entity, (
                With<WidgetryListView<T, K>>,
                Without<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryListView<T, K>>>,
            )>()
            .iter(world)
            .collect::<Vec<_>>();
        for root in roots {
            if let Some(mut style) = world.get_mut::<StyleInvalidation>(root) {
                style.full = true;
            }
        }
        update::<T, K>(world)
    });
}

pub fn apply_owned_list_colors<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
    colors: &bevy_widgetry_theme::WidgetryListViewColors,
) -> Result<(), BevyError> {
    apply_owned_list_colors_with_key::<T, crate::WidgetryListItemId>(world, root, colors)
}
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_theme::WIDGETRY_DARK_THEME;

    fn fixture(count: usize) -> (App, Entity, Vec<Entity>) {
        use crate::{
            WidgetryListModel, WidgetryListViewAppExt, WidgetryListViewPlugin,
            WidgetryListViewRenderer,
        };
        let mut app = bevy_widgetry_test_utils::scene_app();
        bevy_widgetry_test_utils::LogCapture::default().run(|| {
            app.add_plugins(WidgetryListViewPlugin)
                .register_widgetry_list_view::<String>()
                .unwrap();
        });
        let mut model = WidgetryListModel::default();
        for i in 0..10 {
            model.push(i.to_string()).unwrap();
        }
        let source = app.world_mut().spawn(model).id();
        let mut roots = Vec::new();
        for _ in 0..count {
            let root = app.world_mut().spawn_scene(bsn! {
                @WidgetryListView::<String> {
                    @source: source,
                    @item_height: 32.0,
                    @renderer: {WidgetryListViewRenderer::new(|value: &String| bsn_list! { Text({value.clone()}) bevy_widgetry_core::text::WidgetryText })},
                }
            }).unwrap().id();
            let viewport = app.world().get::<Children>(root).unwrap()[0];
            app.world_mut().entity_mut(viewport).insert(ComputedNode {
                size: Vec2::new(200.0, 96.0),
                inverse_scale_factor: 1.0,
                ..default()
            });
            roots.push(root);
        }
        app.update();
        (app, source, roots)
    }

    fn take_work(app: &mut App) -> (usize, usize) {
        let work = std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        (work.roots, work.rows)
    }

    #[test]
    fn stable_views_do_not_resolve_colors_and_hover_only_resolves_one_row() {
        let (mut app, _, roots) = fixture(2);
        let initial = std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        assert_eq!((initial.roots, initial.rows), (2, 6));
        for _ in 0..3 {
            app.update();
        }
        let work = std::mem::take(&mut *app.world_mut().resource_mut::<StyleWork>());
        assert_eq!((work.roots, work.rows), (0, 0));
        let row = app.world().get::<ListRuntime>(roots[0]).unwrap().rows[0];
        app.world_mut().entity_mut(row).insert(Hovered(true));
        app.update();
        let work = app.world().resource::<StyleWork>();
        assert_eq!((work.roots, work.rows), (0, 1));
    }

    #[test]
    fn focus_transfer_refreshes_only_the_old_and_new_view() {
        use bevy::input_focus::FocusCause;
        let (mut app, _, roots) = fixture(3);
        take_work(&mut app);
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(roots[0], FocusCause::Navigated);
        app.update();
        assert_eq!(take_work(&mut app), (1, 3));
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(roots[1], FocusCause::Navigated);
        app.update();
        assert_eq!(take_work(&mut app), (2, 6));
        let colors = WIDGETRY_DARK_THEME.list_view.container;
        assert_eq!(
            *app.world().get::<BorderColor>(roots[0]).unwrap(),
            BorderColor::all(colors.normal.border)
        );
        assert_eq!(
            *app.world().get::<BorderColor>(roots[1]).unwrap(),
            BorderColor::all(colors.focused.border)
        );
        app.update();
        assert_eq!(take_work(&mut app), (0, 0));
    }

    #[test]
    fn owned_palette_handles_local_input_new_content_and_owner_removal() {
        use crate::WidgetryListModel;
        use bevy_widgetry_core::color::WidgetryStyleOwner;
        let (mut app, source, roots) = fixture(1);
        let root = roots[0];
        app.world_mut()
            .entity_mut(root)
            .insert(WidgetryStyleOwner::<WidgetryListView<String>>::new(root));
        let mut colors = WIDGETRY_DARK_THEME.list_view;
        colors.item.hovered.foreground = Color::srgb(0.8, 0.2, 0.4);
        colors.item.normal.foreground = Color::srgb(0.1, 0.7, 0.3);
        apply_owned_list_colors::<String>(app.world_mut(), root, &colors).unwrap();
        take_work(&mut app);
        for _ in 0..3 {
            apply_owned_list_colors::<String>(app.world_mut(), root, &colors).unwrap();
            app.update();
        }
        assert_eq!(take_work(&mut app), (0, 0));
        let row = app.world().get::<ListRuntime>(root).unwrap().rows[0];
        app.world_mut().entity_mut(row).insert(Hovered(true));
        app.update();
        assert_eq!(take_work(&mut app), (0, 1));
        assert_eq!(
            app.world().get::<ResolvedForeground>(row).unwrap().0,
            colors.item.hovered.foreground
        );
        let old_text = app.world().get::<Children>(row).unwrap()[0];
        *app.world_mut()
            .get_mut::<WidgetryListModel<String>>(source)
            .unwrap()
            .get_mut(0)
            .unwrap()
            .unwrap() = "replaced".into();
        app.update();
        assert_eq!(take_work(&mut app), (0, 1));
        let text = app.world().get::<Children>(row).unwrap()[0];
        assert_ne!(text, old_text);
        assert_eq!(app.world().get::<Text>(text).unwrap().0, "replaced");
        assert_eq!(
            app.world().get::<TextColor>(text).unwrap().0,
            colors.item.hovered.foreground
        );
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryStyleOwner<WidgetryListView<String>>>();
        app.update();
        assert_eq!(take_work(&mut app), (1, 3));
        assert_eq!(
            app.world().get::<ResolvedForeground>(row).unwrap().0,
            WIDGETRY_DARK_THEME.list_view.item.hovered.foreground
        );
    }

    #[test]
    fn missing_output_propagates_error_and_retries_after_repair() {
        let (mut app, _, roots) = fixture(1);
        let root = roots[0];
        let row = app.world().get::<ListRuntime>(root).unwrap().rows[0];
        app.world_mut().entity_mut(row).remove::<BackgroundColor>();
        let colors = WIDGETRY_DARK_THEME.list_view;
        bevy_widgetry_test_utils::LogCapture::default().run(|| {
            for _ in 0..2 {
                let error =
                    apply_owned_list_colors::<String>(app.world_mut(), root, &colors).unwrap_err();
                assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            }
        });
        app.world_mut()
            .entity_mut(row)
            .insert((BackgroundColor(Color::WHITE), Hovered(true)));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(row).unwrap().0,
            colors.item.hovered.background
        );
        take_work(&mut app);
        app.update();
        assert_eq!(take_work(&mut app), (0, 0));
    }

    fn colors() -> bevy_widgetry_theme::WidgetryListViewColors {
        let mut colors = WIDGETRY_DARK_THEME.list_view;
        colors.container.normal.border = Color::srgb_u8(1, 0, 0);
        colors.container.focused.border = Color::srgb_u8(2, 0, 0);
        colors.container.disabled.border = Color::srgb_u8(3, 0, 0);
        colors.item.pressed.background = Color::srgb_u8(4, 0, 0);
        colors.item.hovered.background = Color::srgb_u8(5, 0, 0);
        colors.item.selected.background = Color::srgb_u8(6, 0, 0);
        colors.item.normal.foreground = Color::srgb_u8(7, 0, 0);
        colors.item.disabled.foreground = Color::srgb_u8(8, 0, 0);
        colors.item.active_border = Color::srgb_u8(9, 0, 0);
        colors
    }

    #[test]
    fn row_priority_and_active_border_are_orthogonal() {
        let colors = colors();
        for mask in 0..32 {
            let disabled = mask & 1 != 0;
            let pressed = mask & 2 != 0;
            let hovered = mask & 4 != 0;
            let selected = mask & 8 != 0;
            let active = mask & 16 != 0;
            let style = resolve_row(&colors, disabled, pressed, hovered, selected, active);
            assert_eq!(
                style.background,
                if disabled {
                    Color::NONE
                } else if pressed {
                    colors.item.pressed.background
                } else if hovered {
                    colors.item.hovered.background
                } else if selected {
                    colors.item.selected.background
                } else {
                    Color::NONE
                },
                "mask={mask}"
            );
            let row = if disabled {
                colors.item.disabled
            } else if pressed {
                colors.item.pressed
            } else if hovered {
                colors.item.hovered
            } else if selected {
                colors.item.selected
            } else {
                colors.item.normal
            };
            assert_eq!(style.foreground, row.foreground, "mask={mask}");
            assert_eq!(
                style.border,
                if active {
                    if disabled {
                        colors.item.disabled_active_border
                    } else {
                        colors.item.active_border
                    }
                } else {
                    Color::NONE
                },
                "mask={mask}"
            );
        }
    }

    #[test]
    fn root_disabled_overrides_focus() {
        let colors = colors();
        assert_eq!(
            root_border(&colors, false, false),
            colors.container.normal.border
        );
        assert_eq!(
            root_border(&colors, false, true),
            colors.container.focused.border
        );
        assert_eq!(
            root_border(&colors, true, false),
            colors.container.disabled.border
        );
        assert_eq!(
            root_border(&colors, true, true),
            colors.container.disabled.border
        );
    }
}
