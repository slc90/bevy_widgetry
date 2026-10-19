use crate::projection::{TableRuntime, required};
use crate::{
    WidgetryTable, WidgetryTableCell, WidgetryTableColumnHeader, WidgetryTableRowHeader,
    WidgetryTableSelection, WidgetryTableState,
};
use bevy::ecs::entity::EntityHashSet;
use bevy::input_focus::InputFocus;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Selected};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_theme::{WidgetryTableColors, WidgetryThemeMode};

#[cfg(test)]
thread_local! {
    static STYLE_WORK: std::cell::RefCell<(Vec<Entity>, Vec<Entity>)> = const { std::cell::RefCell::new((Vec::new(), Vec::new())) };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Region {
    Table,
    Corner,
    Cell,
    ColumnHeader,
    RowHeader,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct StyledPart {
    root: Entity,
    region: Region,
    selected: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Appearance {
    selection: WidgetryTableSelection,
    cell: Option<WidgetryTableCell>,
    focused: bool,
    range: Option<crate::WidgetryTableSelectionRange>,
}

#[derive(Default)]
struct Pending {
    palette: bool,
    full: bool,
    geometry: bool,
    state: bool,
    parts: EntityHashSet,
    geometry_parts: EntityHashSet,
}

impl Pending {
    fn is_empty(&self) -> bool {
        !self.palette
            && !self.full
            && !self.geometry
            && !self.state
            && self.parts.is_empty()
            && self.geometry_parts.is_empty()
    }
}

#[derive(Component)]
pub(crate) struct TableStyle {
    colors: Option<WidgetryTableColors>,
    appearance: Appearance,
    pending: Pending,
    diagnostics: FailureState,
}

impl Default for TableStyle {
    fn default() -> Self {
        Self {
            colors: None,
            appearance: default(),
            pending: Pending {
                palette: true,
                geometry: true,
                ..default()
            },
            diagnostics: default(),
        }
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_observer(removed_inputs)
        .add_observer(removed_disabled)
        .add_observer(removed_configuration)
        .add_observer(removed_geometry)
        .add_observer(removed_selection)
        .add_systems(
            PostUpdate,
            collect_changes.in_set(WidgetryUiSystems::StyleOwners),
        );
}

pub(super) fn bind_part(world: &mut World, root: Entity, entity: Entity, region: Region) -> Result {
    let part = StyledPart {
        root,
        region,
        selected: false,
    };
    if world
        .get::<StyledPart>(entity)
        .is_none_or(|part| part.root != root || part.region != region)
    {
        world.entity_mut(entity).insert(part);
        let mut style = required(world.get_mut::<TableStyle>(root))?;
        style.pending.parts.insert(entity);
        style.pending.geometry_parts.insert(entity);
    }
    Ok(())
}

pub(super) fn content_changed(world: &mut World, entity: Entity) {
    if let Some(part) = world.get::<StyledPart>(entity).copied()
        && let Some(mut style) = world.get_mut::<TableStyle>(part.root)
    {
        style.pending.parts.insert(entity);
    }
}

fn removed_inputs(
    event: On<
        Remove<(
            Hovered,
            InteractionDisabled,
            BackgroundColor,
            BorderColor,
            ResolvedForeground,
        )>,
    >,
    parts: Query<&StyledPart>,
    mut roots: Query<&mut TableStyle>,
) {
    if let Ok(part) = parts.get(event.entity)
        && let Ok(mut style) = roots.get_mut(part.root)
    {
        style.pending.parts.insert(event.entity);
    }
}

fn removed_selection(
    event: On<Remove<Selected>>,
    parts: Query<&StyledPart>,
    mut roots: Query<&mut TableStyle>,
) {
    if let Ok(part) = parts.get(event.entity)
        && part.selected
        && let Ok(mut style) = roots.get_mut(part.root)
    {
        style.pending.parts.insert(event.entity);
    }
}

fn removed_geometry(
    event: On<Remove<Node>>,
    parts: Query<&StyledPart>,
    mut roots: Query<&mut TableStyle>,
) {
    if let Ok(part) = parts.get(event.entity)
        && let Ok(mut style) = roots.get_mut(part.root)
    {
        style.pending.geometry_parts.insert(event.entity);
    }
}

fn removed_disabled(event: On<Remove<InteractionDisabled>>, mut roots: Query<&mut TableStyle>) {
    if let Ok(mut style) = roots.get_mut(event.entity) {
        style.pending.full = true;
    }
}

fn removed_configuration(
    event: On<
        Remove<(
            crate::colors::ColorState,
            WidgetryTableStyle,
            WidgetryTableState,
        )>,
    >,
    mut roots: Query<&mut TableStyle>,
) {
    if let Ok(mut style) = roots.get_mut(event.entity) {
        style.pending.palette = true;
        style.pending.full = true;
        style.pending.geometry = true;
        style.pending.state = true;
    }
}

fn collect_changes(
    palettes: Query<Entity, (With<TableStyle>, Changed<crate::colors::ColorState>)>,
    geometry: Query<Entity, (With<TableStyle>, Changed<WidgetryTableStyle>)>,
    states: Query<Entity, (With<TableStyle>, Changed<WidgetryTableState>)>,
    projections: Query<(Entity, &WidgetryTableState), Changed<crate::WidgetryTableViewState>>,
    disabled: Query<Entity, (With<TableStyle>, Added<InteractionDisabled>)>,
    parts: Query<
        (Entity, &StyledPart),
        Or<(
            Changed<Hovered>,
            Added<InteractionDisabled>,
            Added<BackgroundColor>,
            Added<BorderColor>,
            Added<ResolvedForeground>,
            Added<Node>,
            Changed<WidgetryTableCell>,
            Changed<WidgetryTableColumnHeader>,
            Changed<WidgetryTableRowHeader>,
        )>,
    >,
    focus: Option<Res<InputFocus>>,
    mut previous_focus: Local<Option<Entity>>,
    mut roots: Query<&mut TableStyle>,
) {
    for root in &palettes {
        if let Ok(mut style) = roots.get_mut(root) {
            style.pending.palette = true;
        }
    }
    for root in &geometry {
        if let Ok(mut style) = roots.get_mut(root) {
            style.pending.geometry = true;
        }
    }
    for root in &states {
        if let Ok(mut style) = roots.get_mut(root) {
            style.pending.state = true;
        }
    }
    for (root, state) in &projections {
        if state.selection() != WidgetryTableSelection::None
            && let Ok(mut style) = roots.get_mut(root)
        {
            style.pending.full = true;
        }
    }
    for root in &disabled {
        if let Ok(mut style) = roots.get_mut(root) {
            style.pending.full = true;
        }
    }
    for (entity, part) in &parts {
        if let Ok(mut style) = roots.get_mut(part.root) {
            style.pending.parts.insert(entity);
        }
    }
    let focused = focus.and_then(|focus| focus.get());
    if *previous_focus != focused {
        for root in previous_focus.iter().copied().chain(focused) {
            if let Ok(mut style) = roots.get_mut(root) {
                style.pending.state = true;
            }
        }
        *previous_focus = focused;
    }
}

#[derive(Clone, Default)]
pub struct WidgetryTableRegionStyle {
    pub padding: UiRect,
    pub border: UiRect,
}

#[derive(Component, Clone)]
pub struct WidgetryTableStyle {
    pub table: WidgetryTableRegionStyle,
    pub column_header: WidgetryTableRegionStyle,
    pub row_header: WidgetryTableRegionStyle,
    pub corner: WidgetryTableRegionStyle,
    pub cell: WidgetryTableRegionStyle,
}

impl Default for WidgetryTableStyle {
    fn default() -> Self {
        let item = WidgetryTableRegionStyle {
            padding: UiRect::horizontal(px(6)),
            border: UiRect::all(px(1)),
        };
        Self {
            table: WidgetryTableRegionStyle {
                border: UiRect::all(px(1)),
                ..default()
            },
            column_header: item.clone(),
            row_header: item.clone(),
            corner: item.clone(),
            cell: item,
        }
    }
}

fn style_shell(
    world: &mut World,
    root: Entity,
    entity: Entity,
    colors: &bevy_widgetry_theme::WidgetryTableRegionColors,
    disabled: bool,
) -> Result<(), BevyError> {
    #[cfg(test)]
    STYLE_WORK.with_borrow_mut(|work| work.1.push(entity));
    let (selected, focused) = region_appearance(world, root, entity, appearance(world, root)?);
    // Record the expected output before Remove so our own deselection does not enqueue a second refresh.
    required(world.get_mut::<StyledPart>(entity))?.selected = selected;
    if selected {
        if world.get::<Selected>(entity).is_none() {
            world.entity_mut(entity).insert(Selected);
        }
    } else if world.get::<Selected>(entity).is_some() {
        world.entity_mut(entity).remove::<Selected>();
    }
    let hovered = world
        .get::<Hovered>(entity)
        .is_some_and(|hovered| hovered.0);
    let disabled = disabled || world.get::<InteractionDisabled>(entity).is_some();
    let state = if disabled {
        colors.disabled
    } else if hovered {
        colors.hovered
    } else if selected {
        colors.selected
    } else {
        colors.normal
    };
    let background = state.background;
    let border = if focused {
        if disabled {
            colors.disabled_focused_border
        } else {
            colors.focused_border
        }
    } else {
        state.border
    };
    let foreground = state.foreground;
    required(world.get_mut::<BackgroundColor>(entity))?.set_if_neq(BackgroundColor(background));
    required(world.get_mut::<BorderColor>(entity))?.set_if_neq(BorderColor::all(border));
    if let Some(mut color) = world.get_mut::<ResolvedForeground>(entity)
        && *color != ResolvedForeground(foreground)
    {
        color.set_if_neq(ResolvedForeground(foreground));
    }
    Ok(())
}

fn apply_geometry(
    world: &mut World,
    entity: Entity,
    style: &WidgetryTableRegionStyle,
) -> Result<(), BevyError> {
    let mut node = required(world.get_mut::<Node>(entity))?;
    if node.padding != style.padding {
        node.padding = style.padding;
    }
    if node.border != style.border {
        node.border = style.border;
    }
    Ok(())
}

pub(crate) fn update_colors<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<(Entity, &TableStyle), With<WidgetryTable<T>>>()
        .iter(world)
        .filter(|(_, style)| !style.pending.is_empty())
        .map(|(entity, _)| entity)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if world.get::<TableRuntime>(root).is_none() {
            continue;
        }
        let mut pending = std::mem::take(&mut required(world.get_mut::<TableStyle>(root))?.pending);
        let result = update_root(world, root, &mut pending);
        let mut style = required(world.get_mut::<TableStyle>(root))?;
        let result = style.diagnostics.observe(
            result,
            |error| widgetry_error!(?root, %error, "Table 区域样式更新失败"),
            || widgetry_info!(?root, "Table 区域样式恢复正常"),
        );
        if let Err(error) = result {
            style.pending.palette |= pending.palette;
            style.pending.full |= pending.full;
            style.pending.geometry |= pending.geometry;
            style.pending.state |= pending.state;
            style.pending.parts.extend(pending.parts);
            style.pending.geometry_parts.extend(pending.geometry_parts);
            failure.get_or_insert(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

fn appearance(world: &World, root: Entity) -> Result<Appearance, BevyError> {
    let state = required(world.get::<WidgetryTableState>(root))?;
    let view = required(world.get::<crate::WidgetryTableViewState>(root))?;
    let geometry = required(world.get::<crate::layout::TableGeometry>(root))?;
    Ok(Appearance {
        range: state
            .selection()
            .resolve(view, geometry.columns.len(), |column| {
                geometry.indices.get(&column).copied()
            }),
        selection: state.selection(),
        cell: state.focused_cell(),
        focused: world
            .get_resource::<InputFocus>()
            .is_some_and(|focus| focus.get() == Some(root)),
    })
}

fn region_appearance(
    world: &World,
    root: Entity,
    entity: Entity,
    appearance: Appearance,
) -> (bool, bool) {
    let position = |row, column| {
        world
            .get::<crate::WidgetryTableViewState>(root)
            .and_then(|view| view.display_index(row))
            .zip(
                world
                    .get::<crate::layout::TableGeometry>(root)
                    .and_then(|geometry| geometry.indices.get(&column).copied()),
            )
    };
    if let Some(cell) = world.get::<WidgetryTableCell>(entity) {
        let selected = appearance
            .range
            .zip(position(cell.row, cell.column))
            .is_some_and(|(range, (row, column))| range.contains(row, column));
        (
            selected,
            appearance.focused && appearance.cell == Some(*cell),
        )
    } else if let Some(header) = world.get::<WidgetryTableColumnHeader>(entity) {
        let selected = matches!(
            appearance.selection,
            WidgetryTableSelection::Column(_) | WidgetryTableSelection::Columns { .. }
        ) && appearance
            .range
            .zip(
                world
                    .get::<crate::layout::TableGeometry>(root)
                    .and_then(|geometry| geometry.indices.get(&header.column).copied()),
            )
            .is_some_and(|(range, column)| range.columns().contains(&column));
        (selected, false)
    } else if let Some(header) = world.get::<WidgetryTableRowHeader>(entity) {
        let selected = matches!(
            appearance.selection,
            WidgetryTableSelection::Row(_) | WidgetryTableSelection::Rows { .. }
        ) && appearance
            .range
            .is_some_and(|range| range.rows().contains(&header.index));
        (selected, false)
    } else {
        (
            world.get::<crate::WidgetryTableCorner>(entity).is_some()
                && appearance.selection == WidgetryTableSelection::All,
            false,
        )
    }
}

fn visible_parts(world: &World, root: Entity) -> Result<Vec<Entity>, BevyError> {
    let runtime = required(world.get::<TableRuntime>(root))?;
    Ok([root, runtime.corner]
        .into_iter()
        .chain(runtime.cells.values().copied())
        .chain(runtime.column_headers.values().copied())
        .chain(runtime.row_headers.values().copied())
        .collect())
}

fn update_root(world: &mut World, root: Entity, pending: &mut Pending) -> Result {
    if pending.palette || required(world.get::<TableStyle>(root))?.colors.is_none() {
        #[cfg(test)]
        STYLE_WORK.with_borrow_mut(|work| work.0.push(root));
        let theme = world.resource::<WidgetryThemeMode>().colors().table;
        let colors = required(world.get::<crate::colors::ColorState>(root))?
            .0
            .resolve(&theme);
        required(world.get_mut::<TableStyle>(root))?.colors = Some(colors);
        pending.palette = false;
        pending.full = true;
    }
    let current = appearance(world, root)?;
    let previous = required(world.get::<TableStyle>(root))?.appearance;
    if pending.full || pending.geometry || (pending.state && previous != current) {
        for entity in visible_parts(world, root)? {
            if pending.full
                || (pending.state
                    && region_appearance(world, root, entity, previous)
                        != region_appearance(world, root, entity, current))
            {
                pending.parts.insert(entity);
            }
            if pending.geometry {
                pending.geometry_parts.insert(entity);
            }
        }
    }
    pending.full = false;
    pending.geometry = false;
    pending.state = false;
    required(world.get_mut::<TableStyle>(root))?.appearance = current;
    let colors = required(required(world.get::<TableStyle>(root))?.colors)?;
    let disabled = world.get::<InteractionDisabled>(root).is_some();
    let parts = pending.parts.iter().copied().collect::<Vec<_>>();
    for entity in parts {
        if !world.entities().contains(entity) {
            pending.parts.remove(&entity);
            continue;
        }
        let part = *required(world.get::<StyledPart>(entity))?;
        if part.root != root {
            pending.parts.remove(&entity);
            continue;
        }
        let colors = match part.region {
            Region::Table => colors.table,
            Region::Corner => colors.corner,
            Region::Cell => colors.cell,
            Region::ColumnHeader => colors.column_header,
            Region::RowHeader => colors.row_header,
        };
        style_shell(world, root, entity, &colors, disabled)?;
        pending.parts.remove(&entity);
    }
    if !pending.geometry_parts.is_empty() {
        let config = required(world.get::<WidgetryTableStyle>(root))?.clone();
        let parts = pending.geometry_parts.iter().copied().collect::<Vec<_>>();
        for entity in parts {
            if !world.entities().contains(entity) {
                pending.geometry_parts.remove(&entity);
                continue;
            }
            let part = *required(world.get::<StyledPart>(entity))?;
            if part.root != root {
                pending.geometry_parts.remove(&entity);
                continue;
            }
            let config = match part.region {
                Region::Table => &config.table,
                Region::Corner => &config.corner,
                Region::Cell => &config.cell,
                Region::ColumnHeader => &config.column_header,
                Region::RowHeader => &config.row_header,
            };
            apply_geometry(world, entity, config)?;
            pending.geometry_parts.remove(&entity);
        }
    }
    Ok(())
}

pub(crate) fn refresh_theme<T: Send + Sync + 'static>(
    _event: On<bevy_widgetry_theme::WidgetryThemeChanged>,
    mut roots: Query<&mut TableStyle, With<WidgetryTable<T>>>,
    mut commands: Commands,
) {
    for mut style in &mut roots {
        style.pending.palette = true;
    }
    commands.queue(update_colors::<T>);
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::*;
    use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};

    #[derive(Resource, Default)]
    struct HoverChange(Option<(Entity, Option<bool>)>);

    fn apply_hover(world: &mut World) {
        if let Some((entity, hovered)) = world.resource_mut::<HoverChange>().0.take() {
            if let Some(hovered) = hovered {
                world.entity_mut(entity).insert(Hovered(hovered));
            } else {
                world.entity_mut(entity).remove::<Hovered>();
            }
        }
    }

    fn take_work() -> (Vec<Entity>, Vec<Entity>) {
        STYLE_WORK.with_borrow_mut(std::mem::take)
    }

    fn fixture() -> (App, Entity, [Entity; 2]) {
        let mut app = scene_app();
        add_ui_plugins(&mut app);
        spawn_ui_camera(&mut app, UVec2::new(600, 400), 1.0);
        app.register_widgetry_table::<u32>();
        app.init_resource::<HoverChange>()
            .add_systems(Update, apply_hover);
        app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| bsn_list! { Text({value.clone()}) bevy_widgetry_core::text::WidgetryText })).unwrap();
        app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &String| bsn_list! { Text({value.clone()}) bevy_widgetry_core::text::WidgetryText })).unwrap();
        let mut model = WidgetryTableModel::default();
        for row in 0..3 {
            model.push_row(row).unwrap();
        }
        for column in 0..3 {
            model
                .push_column(WidgetryTableColumn::new(
                    WidgetryTableHeaderValue::new(format!("C{column}")),
                    column,
                    |row: &u32, column: &u32| {
                        WidgetryTableCellValue::new(format!("{row}/{column}"))
                    },
                ))
                .unwrap();
        }
        let source = app.world_mut().spawn(model).id();
        let roots = std::array::from_fn(|_| {
            app.world_mut().spawn_scene(bsn! { @WidgetryTable::<u32> { @source: source } Node { width: px(286), height: px(144) } }).unwrap().id()
        });
        for _ in 0..3 {
            app.update();
        }
        (app, source, roots)
    }

    #[test]
    fn stable_tables_skip_palette_and_visible_region_style_work() {
        let (mut app, _, _) = fixture();
        STYLE_WORK.with_borrow_mut(|work| {
            work.0.clear();
            work.1.clear();
        });
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            STYLE_WORK.with_borrow(|work| (work.0.len(), work.1.len())),
            (0, 0)
        );
    }

    #[test]
    fn local_hover_and_disabled_removal_only_recolor_the_target() {
        let (mut app, _, roots) = fixture();
        let cell = *app
            .world()
            .get::<TableRuntime>(roots[0])
            .unwrap()
            .cells
            .values()
            .next()
            .unwrap();
        for (hovered, expected) in [
            (
                Some(true),
                WidgetryThemeMode::Dark.colors().table.cell.hovered,
            ),
            (None, WidgetryThemeMode::Dark.colors().table.cell.normal),
        ] {
            take_work();
            app.world_mut().resource_mut::<HoverChange>().0 = Some((cell, hovered));
            app.update();
            assert_eq!(take_work(), (vec![], vec![cell]));
            assert_eq!(
                app.world().get::<BackgroundColor>(cell).unwrap().0,
                expected.background
            );
        }
        app.world_mut().entity_mut(cell).insert(InteractionDisabled);
        app.update();
        assert_eq!(take_work(), (vec![], vec![cell]));
        assert_eq!(
            app.world().get::<BackgroundColor>(cell).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .table
                .cell
                .disabled
                .background
        );
        app.world_mut()
            .entity_mut(cell)
            .remove::<InteractionDisabled>();
        app.update();
        assert_eq!(take_work(), (vec![], vec![cell]));
        assert_eq!(
            app.world().get::<BackgroundColor>(cell).unwrap().0,
            WidgetryThemeMode::Dark
                .colors()
                .table
                .cell
                .normal
                .background
        );
    }

    #[test]
    fn selection_regions_focus_and_style_geometry_use_their_own_invalidation() {
        let (mut app, source, roots) = fixture();
        let row = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .row_id(0)
            .unwrap();
        let column = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_id(0)
            .unwrap();
        let cell = WidgetryTableCell { row, column };
        let entity = app.world().get::<TableRuntime>(roots[0]).unwrap().cells[&(row, column)];
        for selection in [
            WidgetryTableSelection::Row(row),
            WidgetryTableSelection::Column(column),
            WidgetryTableSelection::Cell { row, column },
            WidgetryTableSelection::None,
        ] {
            let previous = appearance(app.world(), roots[0]).unwrap();
            WidgetryTable::<u32>::set_selection(app.world_mut(), roots[0], selection).unwrap();
            let current = appearance(app.world(), roots[0]).unwrap();
            let expected = visible_parts(app.world(), roots[0])
                .unwrap()
                .into_iter()
                .filter(|part| {
                    region_appearance(app.world(), roots[0], *part, previous)
                        != region_appearance(app.world(), roots[0], *part, current)
                })
                .collect::<EntityHashSet>();
            take_work();
            app.update();
            let (palettes, parts) = take_work();
            assert!(palettes.is_empty());
            assert_eq!(parts.into_iter().collect::<EntityHashSet>(), expected);
            for part in visible_parts(app.world(), roots[0]).unwrap() {
                assert_eq!(
                    app.world().get::<Selected>(part).is_some(),
                    region_appearance(app.world(), roots[0], part, current).0
                );
            }
            assert_eq!(
                app.world()
                    .get::<WidgetryTableState>(roots[1])
                    .unwrap()
                    .selection(),
                WidgetryTableSelection::None
            );
        }
        WidgetryTable::<u32>::set_focused_cell(app.world_mut(), roots[0], Some(cell)).unwrap();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(roots[0], bevy::input_focus::FocusCause::Navigated);
        app.update();
        assert_eq!(take_work(), (vec![], vec![entity]));
        assert_eq!(
            app.world().get::<BorderColor>(entity).unwrap().top,
            WidgetryThemeMode::Dark.colors().table.cell.focused_border
        );
        let other = app.world().get::<TableRuntime>(roots[1]).unwrap().cells[&(row, column)];
        WidgetryTable::<u32>::set_focused_cell(app.world_mut(), roots[1], Some(cell)).unwrap();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(roots[1], bevy::input_focus::FocusCause::Navigated);
        app.update();
        let (palettes, parts) = take_work();
        assert!(palettes.is_empty());
        assert_eq!(
            parts.into_iter().collect::<EntityHashSet>(),
            [entity, other].into_iter().collect()
        );
        assert_eq!(
            app.world().get::<BorderColor>(entity).unwrap().top,
            WidgetryThemeMode::Dark.colors().table.cell.normal.border
        );
        app.world_mut()
            .get_mut::<WidgetryTableStyle>(roots[0])
            .unwrap()
            .cell
            .padding = UiRect::all(px(3));
        app.update();
        assert_eq!(take_work(), (vec![], vec![]));
        assert_eq!(
            app.world().get::<Node>(entity).unwrap().padding,
            UiRect::all(px(3))
        );
        assert_eq!(
            app.world().get::<Node>(other).unwrap().padding,
            WidgetryTableStyle::default().cell.padding
        );
    }

    #[test]
    fn palette_and_output_failures_recover_without_repeated_diagnostics() {
        use bevy::ecs::{error::Severity, schedule::SingleThreadedExecutor};
        use bevy::log::tracing::Level;
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, switch_theme};
        let logs = LogCapture::default();
        let (mut app, source, roots) = logs.run(fixture);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
        let row = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .row_id(0)
            .unwrap();
        let column = app
            .world()
            .get::<WidgetryTableModel<u32>>(source)
            .unwrap()
            .column_id(0)
            .unwrap();
        let cell = app.world().get::<TableRuntime>(roots[0]).unwrap().cells[&(row, column)];
        let mut colors = WidgetryTableColorOverrides::default();
        let background = Color::srgb_u8(17, 34, 51);
        colors.cell.normal.background = Some(background);
        take_work();
        WidgetryTableColorOverrides::set_in_world(app.world_mut(), roots[0], colors).unwrap();
        app.update();
        let (palettes, parts) = take_work();
        assert_eq!(palettes, vec![roots[0]]);
        assert!(
            parts
                .iter()
                .all(|part| app.world().get::<StyledPart>(*part).unwrap().root == roots[0])
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(cell).unwrap().0,
            background
        );
        let border = app
            .world_mut()
            .entity_mut(cell)
            .take::<BorderColor>()
            .unwrap();
        let errors = ErrorCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.update();
                app.update();
            })
        });
        let failures = errors.take();
        assert_eq!(failures.len(), 2);
        assert!(
            failures
                .iter()
                .all(|failure| failure.severity() == Severity::Error)
        );
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == Level::ERROR)
                .count(),
            1
        );
        app.world_mut().entity_mut(cell).insert(border);
        logs.run(|| errors.run(|| app.update()));
        assert!(errors.take().is_empty());
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.level == Level::INFO
                    && record
                        .fields
                        .get("message")
                        .is_some_and(|message| message.contains("区域样式恢复")))
                .count(),
            1
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(cell).unwrap().0,
            background
        );
        take_work();
        app.update();
        assert_eq!(take_work(), (vec![], vec![]));
        WidgetryTable::<u32>::set_selection(
            app.world_mut(),
            roots[0],
            WidgetryTableSelection::Cell { row, column },
        )
        .unwrap();
        app.update();
        take_work();
        app.world_mut().entity_mut(cell).remove::<Selected>();
        app.update();
        assert!(app.world().get::<Selected>(cell).is_some());
        assert_eq!(take_work(), (vec![], vec![cell]));
        WidgetryTableColorOverrides::clear_in_world(app.world_mut(), roots[0]).unwrap();
        switch_theme(&mut app, WidgetryThemeMode::Light);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(cell).unwrap().0,
            WidgetryThemeMode::Light
                .colors()
                .table
                .cell
                .selected
                .background
        );
        let other = app.world().get::<TableRuntime>(roots[1]).unwrap().cells[&(row, column)];
        assert_eq!(
            app.world().get::<BackgroundColor>(other).unwrap().0,
            WidgetryThemeMode::Light
                .colors()
                .table
                .cell
                .normal
                .background
        );
        take_work();
        app.world_mut().despawn(roots[0]);
        app.update();
        assert_eq!(take_work(), (vec![], vec![]));
    }
}
