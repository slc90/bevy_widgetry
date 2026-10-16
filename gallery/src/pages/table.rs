use crate::assets::GalleryIcon;
use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{Language, TextKey, localized, localized_args, tr, tr_args};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::{InteractionDisabled, ScrollPosition, UiSystems};
use bevy::ui_widgets::Activate;
use bevy::window::RequestRedraw;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::table::{
    WidgetryTable, WidgetryTableAppExt, WidgetryTableBody, WidgetryTableCell,
    WidgetryTableCellRenderer, WidgetryTableCellValue, WidgetryTableColumn,
    WidgetryTableColumnWidth, WidgetryTableEvent, WidgetryTableEventKind,
    WidgetryTableHeaderRenderer, WidgetryTableHeaderValue, WidgetryTableLayout, WidgetryTableModel,
    WidgetryTableSelection, WidgetryTableState, WidgetryTableStyle,
};

pub(crate) struct TableDemoPlugin;

pub(crate) fn color_examples(world: &World) -> impl Scene + use<> {
    let sources = world.resource::<TableDemoSources>();
    let source = sources.sources[0];
    let layout = sources.layouts[0].clone();
    let view = move || bsn! { @WidgetryTable::<DemoRow> { @source: source, @layout: {layout.clone()} } Node { width: percent(100), height: px(160) } };
    crate::color_showcase::pair(
        TextKey::Table,
        TextKey::OnlyCellSelectedBackground0F766EMovePointer,
        view(),
        view(),
        |world, entity, apply| {
            use bevy_widgetry::table::WidgetryTableColorOverrides;
            if !apply {
                return WidgetryTableColorOverrides::clear_in_world(world, entity);
            }
            let mut colors = WidgetryTableColorOverrides::default();
            colors.cell.selected.background = Some(crate::color_showcase::TEAL);
            WidgetryTableColorOverrides::set_in_world(world, entity, colors)
        },
    )
}

#[derive(Resource, Clone, Reflect)]
#[reflect(Resource)]
pub(crate) struct TableDemoSources {
    pub(crate) sources: [Entity; 8],
    #[reflect(ignore)]
    layouts: [WidgetryTableLayout; 8],
    next_shared_row: u32,
}

struct DemoRow {
    index: u32,
    label: TableLabel,
    number: u32,
    enabled: bool,
    progress: f32,
    edits: u32,
}

#[derive(Reflect)]
struct ProgressValue(f32);

#[derive(Reflect)]
struct StarValue;

#[derive(Reflect)]
struct CustomHeader(TableLabel);

#[derive(Reflect, Clone)]
enum TableLabel {
    Key(TextKey),
    Item(u32),
    Updated(u32),
    Column(usize),
}

impl TableLabel {
    fn scene(&self) -> impl Scene + use<> {
        let (key, args) = match self {
            Self::Key(key) => (*key, vec![]),
            Self::Item(index) => (TextKey::TableItem, vec![("index", format!("{index:02}"))]),
            Self::Updated(edits) => (TextKey::TableUpdated, vec![("edits", edits.to_string())]),
            Self::Column(index) => (TextKey::TableColumn, vec![("index", format!("{index:02}"))]),
        };
        localized_args(key, args)
    }
}

#[derive(Component)]
struct TableDemo;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Demo {
    Basic,
    CellValue,
    Header,
    Selection,
    Layout,
    Virtualization,
    Disabled,
    Shared,
}

#[derive(Component)]
struct DemoNav(Demo);

#[derive(Component)]
struct DemoPanel(Demo);

#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct DemoTable(Demo, usize);

#[derive(Component, Default)]
struct DemoEvents {
    selection: usize,
    resize_start: usize,
    resize_changes: usize,
    resize_end: usize,
    resize_cancel: usize,
}

#[derive(Component)]
struct DemoStatus {
    source: Entity,
    view: DemoTable,
}

#[derive(Component, Clone, Copy)]
struct DemoAction {
    source: Entity,
    view: DemoTable,
    kind: Action,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    SelectFirst,
    ClearSelection,
    UpdateRow,
    ReplaceHeader,
    ToggleDisabled,
    ResizeViewport,
    InsertRow,
    RemoveRow,
    MoveRow,
}

pub(crate) fn scene(sources: TableDemoSources) -> impl Scene {
    let demos = [
        Demo::Basic,
        Demo::CellValue,
        Demo::Header,
        Demo::Selection,
        Demo::Layout,
        Demo::Virtualization,
        Demo::Disabled,
        Demo::Shared,
    ];
    let buttons = demos
        .into_iter()
        .map(|demo| {
            bsn! {
                @WidgetryButton
                template(move |_| Ok(DemoNav(demo)))
                template(move |_| Ok(Name::new(format!("TableDemoNav{demo:?}"))))
                on(change_demo)
                Children [@localized(demo.title()) bevy_widgetry::text::WidgetryText]
            }
        })
        .collect::<Vec<_>>();
    let panels = demos
        .into_iter()
        .enumerate()
        .map(|(index, demo)| -> Box<dyn SceneList> {
            let source = sources.sources[index];
            let layout = sources.layouts[index].clone();
            if demo == Demo::Shared {
                Box::new(bsn_list! { @shared_panel(source, layout) })
            } else {
                Box::new(bsn_list! { @panel(source, layout, demo) })
            }
        })
        .collect::<Vec<_>>();
    bsn! {
        template(|_| Ok(TableDemo))
        Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, row_gap: px(16) }
        Children [
            @localized(TextKey::Table) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(28.0) }--
            Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap } Children [{buttons}]--
            Node { flex_grow: 1.0, min_height: px(0), min_width: px(0) } Children [{panels}]
        ]
    }
}

fn panel(source: Entity, layout: WidgetryTableLayout, demo: Demo) -> impl Scene {
    let controls = [
        (TextKey::SelectFirstApi, Action::SelectFirst),
        (TextKey::ClearSelectionApi, Action::ClearSelection),
        (TextKey::UpdateFirstRow, Action::UpdateRow),
        (TextKey::ReplaceFirstHeader, Action::ReplaceHeader),
        (TextKey::EnableDisable, Action::ToggleDisabled),
        (TextKey::ResizeViewport, Action::ResizeViewport),
    ]
    .into_iter()
    .map(|(label, kind)| {
        bsn! {
            @WidgetryButton
            template(move |_| Ok(DemoAction { source, view: DemoTable(demo, 0), kind }))
            template(move |_| Ok(Name::new(format!("Table{demo:?}{kind:?}"))))
            on(operate)
            Children [@localized(label) bevy_widgetry::text::WidgetryText]
        }
    })
    .collect::<Vec<_>>();
    let table: Box<dyn SceneList> = Box::new(bsn_list! {
        @WidgetryTable::<DemoRow> { @source: source, @layout: {layout.clone()} }
        template(move |_| Ok(Name::new(format!("TableView{demo:?}"))))
        template(|_| Ok(DemoEvents::default())) TabIndex({if demo == Demo::Basic {0} else {-1}})
        Node { width: percent(100), height: px(440), flex_shrink: 0.0 }
        template(move |context| {
            if demo == Demo::Disabled { context.entity.insert(InteractionDisabled); }
            Ok(DemoTable(demo, 0))
        })
    });
    bsn! {
        template(move |_| Ok(DemoPanel(demo)))
        Node {
            width: percent(100), height: percent(100), flex_direction: FlexDirection::Column,
            row_gap: px(12), min_width: px(0),
            display: {if demo == Demo::Basic {Display::Flex} else {Display::None}},
        }
        Children [
            @localized(demo.description()) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(16.0) }--
            template(move |_| Ok(DemoStatus { source, view: DemoTable(demo, 0) })) template(move |_| Ok(Name::new(format!("TableStatus{demo:?}")))) Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(14.0) }--
            {table}--
            Node {column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap} Children [{controls}]
        ]
    }
}

fn shared_panel(source: Entity, layout: WidgetryTableLayout) -> impl Scene {
    let mut compact = layout.clone();
    compact.row_height = 28.0;
    compact.default_column_width = WidgetryTableColumnWidth::Fixed(120.0);
    bsn! {
        template(|_| Ok(DemoPanel(Demo::Shared)))
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(12), min_width: px(0), display: Display::None }
        Children [
            @localized(TextKey::TableSharedDescription) bevy_widgetry::text::WidgetryText--
            Node { width: percent(100), column_gap: px(16), align_items: AlignItems::Start } Children [
                @shared_view(source, layout, 0)--
                @shared_view(source, compact, 1)
            ]
        ]
    }
}

fn shared_view(source: Entity, layout: WidgetryTableLayout, side: usize) -> impl Scene {
    let name = if side == 0 { "SharedA" } else { "SharedB" };
    let view = DemoTable(Demo::Shared, side);
    let controls = [
        (TextKey::SelectFirstApi, Action::SelectFirst),
        (TextKey::ClearSelectionApi, Action::ClearSelection),
        (TextKey::UpdateFirstRow, Action::UpdateRow),
        (TextKey::TableInsertRow, Action::InsertRow),
        (TextKey::RemoveFirst, Action::RemoveRow),
        (TextKey::MoveFirstLast, Action::MoveRow),
        (TextKey::EnableDisable, Action::ToggleDisabled),
    ]
    .into_iter()
    .map(move |(label, kind)| {
        bsn! {
            @WidgetryButton
            template(move |_| Ok(DemoAction { source, view, kind }))
            Name({format!("Table{name}{kind:?}")}) on(operate)
            Children [@localized(label) bevy_widgetry::text::WidgetryText]
        }
    })
    .collect::<Vec<_>>();
    let mut style = WidgetryTableStyle::default();
    if side == 1 {
        style.cell.padding = UiRect::all(px(4));
        style.cell.border = UiRect::all(px(2));
    }
    bsn! {
        Node { flex_basis: percent(50), flex_grow: 1.0, min_width: px(0), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            @localized(if side == 0 {TextKey::TableSharedA} else {TextKey::TableSharedB}) bevy_widgetry::text::WidgetryText--
            template(move |_| Ok(DemoStatus { source, view })) Name({format!("TableStatus{name}")}) Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(13.0) }--
            @WidgetryTable::<DemoRow> { @source: source, @layout: {layout.clone()}, @style: {style.clone()} }
            template(move |_| Ok(view)) template(|_| Ok(DemoEvents::default()))
            Name({format!("TableView{name}")}) TabIndex(-1)
            Node { width: percent(100), height: px(240), flex_shrink: 0.0 }--
            Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap } Children [{controls}]
        ]
    }
}

fn change_demo(
    event: On<Activate>,
    buttons: Query<&DemoNav>,
    mut panels: Query<(&DemoPanel, &mut Node)>,
    mut tables: Query<(&DemoTable, &mut TabIndex)>,
) {
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    for (panel, mut node) in &mut panels {
        node.display = if panel.0 == button.0 {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (table, mut index) in &mut tables {
        index.0 = if table.0 == button.0 { 0 } else { -1 };
    }
    info!(demo = ?button.0, "切换 Table 示例");
}

fn view(world: &mut World, target: DemoTable) -> Result<Entity, BevyError> {
    world
        .query::<(Entity, &DemoTable)>()
        .iter(world)
        .find(|(_, table)| **table == target)
        .map(|(entity, _)| entity)
        .ok_or_else(|| {
            error!(demo = ?target.0, side = target.1, "Gallery Table 示例缺少 View");
            BevyError::error("Gallery Table View missing")
        })
}

fn operate(event: On<Activate>, actions: Query<&DemoAction>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world.get_entity(action.source).is_err() {
            return Ok(());
        }
        let root = view(world, action.view)?;
        match action.kind {
            Action::SelectFirst => {
                let model = world
                    .get::<WidgetryTableModel<DemoRow>>(action.source)
                    .ok_or_else(|| {
                        error!(source = ?action.source, "Gallery Table 示例缺少 Model");
                        BevyError::error("Gallery Table Model missing")
                    })?;
                if let Some((row, column)) = model.row_id(0).zip(model.column_id(0)) {
                    let changed = WidgetryTable::<DemoRow>::set_selection(
                        world,
                        root,
                        WidgetryTableSelection::Cell { row, column },
                    )?;
                    info!(?root, changed, "程序化设置 Table selection");
                }
            }
            Action::ClearSelection => {
                let changed = WidgetryTable::<DemoRow>::set_selection(
                    world,
                    root,
                    WidgetryTableSelection::None,
                )?;
                info!(?root, changed, "程序化清除 Table selection");
            }
            Action::UpdateRow | Action::ReplaceHeader => {
                let mut model = world
                    .get_mut::<WidgetryTableModel<DemoRow>>(action.source)
                    .ok_or_else(|| {
                        error!(source = ?action.source, "Gallery Table 示例缺少 Model");
                        BevyError::error("Gallery Table Model missing")
                    })?;
                if matches!(action.kind, Action::UpdateRow) {
                    if let Some(row) = model.row_mut(0)? {
                        row.edits += 1;
                        row.label = TableLabel::Updated(row.edits);
                        row.number += 1;
                        row.enabled = !row.enabled;
                        row.progress = (row.progress + 0.15) % 1.0;
                        info!(?root, edits = row.edits, "更新 Table 首行内容");
                    }
                } else {
                    model.set_header(
                        0,
                        WidgetryTableHeaderValue::new(CustomHeader(TableLabel::Key(
                            TextKey::TableUpdatedHeader,
                        ))),
                    )?;
                    info!(?root, "替换 Table 首列 Header");
                }
            }
            Action::ToggleDisabled => {
                let disabled = world.get::<InteractionDisabled>(root).is_none();
                if disabled {
                    world.entity_mut(root).insert(InteractionDisabled);
                } else {
                    world.entity_mut(root).remove::<InteractionDisabled>();
                }
                info!(?root, disabled, "设置 Table 整体 disabled");
            }
            Action::InsertRow | Action::RemoveRow | Action::MoveRow => {
                let index = world.resource::<TableDemoSources>().next_shared_row;
                let mut model = world
                    .get_mut::<WidgetryTableModel<DemoRow>>(action.source)
                    .ok_or_else(|| {
                        error!(source = ?action.source, "Gallery Table 示例缺少 Model");
                        BevyError::error("Gallery Table Model missing")
                    })?;
                match action.kind {
                    Action::InsertRow => {
                        model.insert_row(
                            0,
                            DemoRow {
                                index,
                                label: TableLabel::Item(index),
                                number: index * 17 + 3,
                                enabled: true,
                                progress: 0.5,
                                edits: 0,
                            },
                        )?;
                        world.resource_mut::<TableDemoSources>().next_shared_row += 1;
                    }
                    Action::RemoveRow => {
                        model.remove_row(0);
                    }
                    Action::MoveRow if model.row_count() > 1 => {
                        let last = model.row_count() - 1;
                        model.move_row(0, last);
                    }
                    _ => {}
                }
                info!(?root, action = ?action.kind, "修改共享 Table 行");
            }
            Action::ResizeViewport => {
                let mut node = world.get_mut::<Node>(root).ok_or_else(|| {
                    error!(?root, "Gallery Table 示例缺少 Node");
                    BevyError::error("Gallery Table Node missing")
                })?;
                node.width = if node.width == percent(100) {
                    px(720)
                } else {
                    percent(100)
                };
                info!(?root, width = ?node.width, "设置 Table viewport width");
            }
        }
        Ok(())
    });
}

fn on_table_event(
    event: On<WidgetryTableEvent>,
    mut views: Query<(&mut DemoEvents, &WidgetryTableLayout)>,
) {
    let Ok((mut counts, layout)) = views.get_mut(event.entity) else {
        return;
    };
    match event.kind {
        WidgetryTableEventKind::ColumnSelected(_)
        | WidgetryTableEventKind::RowSelected(_)
        | WidgetryTableEventKind::CellSelected { .. }
        | WidgetryTableEventKind::SelectionCleared => {
            counts.selection += 1;
            info!(view = ?event.entity, selection = ?event.kind, "Table selection 变化");
        }
        WidgetryTableEventKind::ColumnResizeStart(_) => counts.resize_start += 1,
        WidgetryTableEventKind::ColumnResized { .. } => counts.resize_changes += 1,
        WidgetryTableEventKind::ColumnResizeEnd(column) => {
            counts.resize_end += 1;
            info!(view = ?event.entity, ?column, width = ?layout.column_widths().get(&column), "结束 Table Column resize");
        }
        WidgetryTableEventKind::ColumnResizeCancel(column) => {
            counts.resize_cancel += 1;
            info!(view = ?event.entity, ?column, width = ?layout.column_widths().get(&column), "取消 Table Column resize");
        }
    }
}

fn owner(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryTable<DemoRow>>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn update_status(world: &mut World) -> Result<(), BevyError> {
    let language = *world.resource::<Language>();
    let statuses = world
        .query::<(Entity, &DemoStatus)>()
        .iter(world)
        .filter(|(entity, _)| {
            let mut current = *entity;
            loop {
                if world
                    .get::<Node>(current)
                    .is_some_and(|node| node.display == Display::None)
                {
                    return false;
                }
                let Some(parent) = world.get::<ChildOf>(current) else {
                    return true;
                };
                current = parent.parent();
            }
        })
        .map(|(entity, status)| (entity, status.source, status.view))
        .collect::<Vec<_>>();
    if statuses.is_empty() {
        return Ok(());
    }
    let cells = world
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(world)
        .filter_map(|(entity, cell)| owner(world, entity).map(|root| (root, *cell)))
        .collect::<Vec<_>>();
    for (entity, source, target) in statuses {
        let root = view(world, target)?;
        let model = world
            .get::<WidgetryTableModel<DemoRow>>(source)
            .ok_or_else(|| {
                error!(?source, "Gallery Table status 缺少 Model");
                BevyError::error("Gallery Table status Model missing")
            })?;
        let state = world.get::<WidgetryTableState>(root).ok_or_else(|| {
            error!(?root, "Gallery Table status 缺少 logical state");
            BevyError::error("Gallery Table status state missing")
        })?;
        let events = world.get::<DemoEvents>(root).ok_or_else(|| {
            error!(?root, "Gallery Table status 缺少 event state");
            BevyError::error("Gallery Table status events missing")
        })?;
        let indices = cells
            .iter()
            .filter(|(owner, _)| *owner == root)
            .filter_map(|(_, cell)| {
                model
                    .row_index(cell.row)
                    .zip(model.column_index(cell.column))
            })
            .collect::<Vec<_>>();
        let cursor = state.focused_cell().and_then(|cell| {
            model
                .row_index(cell.row)
                .zip(model.column_index(cell.column))
        });
        let scroll = world
            .get::<Children>(root)
            .and_then(|children| {
                children
                    .iter()
                    .find(|&child| world.get::<WidgetryTableBody>(child).is_some())
            })
            .and_then(|body| world.get::<ScrollPosition>(body))
            .map(|position| position.0)
            .unwrap_or_default();
        let label = tr_args(
            language,
            TextKey::TableStatus,
            &[
                ("rows", model.row_count().to_string()),
                ("columns", model.column_count().to_string()),
                ("cells", indices.len().to_string()),
                (
                    "row_start",
                    format!("{:?}", indices.iter().map(|pair| pair.0).min()),
                ),
                (
                    "row_end",
                    format!("{:?}", indices.iter().map(|pair| pair.0).max()),
                ),
                (
                    "column_start",
                    format!("{:?}", indices.iter().map(|pair| pair.1).min()),
                ),
                (
                    "column_end",
                    format!("{:?}", indices.iter().map(|pair| pair.1).max()),
                ),
                ("selection", format!("{:?}", state.selection())),
                ("cursor", format!("{cursor:?}")),
                ("scroll", format!("{:.1}, {:.1}", scroll.x, scroll.y)),
                (
                    "state",
                    tr(
                        language,
                        if world.get::<InteractionDisabled>(root).is_some() {
                            TextKey::Disabled
                        } else {
                            TextKey::Enabled
                        },
                    )
                    .into(),
                ),
                ("select", events.selection.to_string()),
                ("start", events.resize_start.to_string()),
                ("change", events.resize_changes.to_string()),
                ("end", events.resize_end.to_string()),
                ("cancel", events.resize_cancel.to_string()),
            ],
        );
        if let Some(mut text) = world.get_mut::<Text>(entity)
            && text.0 != label
        {
            text.0 = label;
            // Status reads the range after Layout.
            // Request another frame to prepare new Text before a reactive App can stop updating.
            world.write_message(RequestRedraw);
        }
    }
    Ok(())
}

fn column(index: usize, custom: bool, virtualized: bool) -> WidgetryTableColumn<DemoRow> {
    let label = if virtualized {
        TableLabel::Column(index)
    } else {
        TableLabel::Key(
            [
                TextKey::TableName,
                TextKey::TableNumber,
                TextKey::TableBool,
                TextKey::TableProgress,
                TextKey::Icon,
            ][index % 5],
        )
    };
    let header = if custom {
        WidgetryTableHeaderValue::new(CustomHeader(label))
    } else {
        WidgetryTableHeaderValue::new(label)
    };
    WidgetryTableColumn::new(header, index, move |row: &DemoRow, index: &usize| {
        if virtualized {
            return WidgetryTableCellValue::new(format!(
                "R{:04}/C{:02} ({})",
                row.index, index, row.edits
            ));
        }
        match index % 5 {
            0 => WidgetryTableCellValue::new(row.label.clone()),
            1 => WidgetryTableCellValue::new(row.number),
            2 => WidgetryTableCellValue::new(row.enabled),
            3 => WidgetryTableCellValue::new(ProgressValue(row.progress)),
            _ => WidgetryTableCellValue::new(StarValue),
        }
    })
}

fn sources(world: &mut World) -> Result<TableDemoSources, BevyError> {
    let mut models = std::array::from_fn::<_, 8, _>(|_| WidgetryTableModel::default());
    let mut layouts = std::array::from_fn(|_| {
        let mut layout = WidgetryTableLayout::default();
        layout.row_height = 34.0;
        layout.default_column_width = WidgetryTableColumnWidth::Fixed(180.0);
        layout
    });
    for (index, model) in models.iter_mut().enumerate() {
        let virtualized = index == 5;
        for row in 0..if virtualized {
            2000
        } else if index == 7 {
            64
        } else {
            24
        } {
            model.push_row(DemoRow {
                index: row,
                label: TableLabel::Item(row),
                number: row * 17 + 3,
                enabled: row % 2 == 0,
                progress: (row % 11) as f32 / 10.0,
                edits: 0,
            })?;
        }
        let columns = if virtualized {
            40
        } else if index == 7 {
            8
        } else if index == 0 || index == 3 || index == 4 {
            3
        } else {
            5
        };
        for col in 0..columns {
            model.push_column(column(col, index == 2, virtualized))?;
        }
        if index == 4 {
            layouts[index].default_column_width = WidgetryTableColumnWidth::Flexible(1.0);
            if let Some(first) = model.column_id(0) {
                layouts[index] = layouts[index]
                    .clone()
                    .with_column_width(first, WidgetryTableColumnWidth::Fixed(180.0));
            }
            if let Some(second) = model.column_id(1) {
                layouts[index] = layouts[index]
                    .clone()
                    .with_column_width(second, WidgetryTableColumnWidth::Flexible(2.0));
            }
        }
    }
    let sources = models.map(|model| world.spawn(model).id());
    Ok(TableDemoSources {
        sources,
        layouts,
        next_shared_row: 64,
    })
}

impl Demo {
    fn title(self) -> TextKey {
        match self {
            Self::Basic => TextKey::Basic,
            Self::CellValue => TextKey::Cellvalue,
            Self::Header => TextKey::Header,
            Self::Selection => TextKey::Selection,
            Self::Layout => TextKey::ColumnLayout,
            Self::Virtualization => TextKey::Virtualization,
            Self::Disabled => TextKey::Disabled,
            Self::Shared => TextKey::TableShared,
        }
    }

    fn description(self) -> TextKey {
        match self {
            Self::Basic => TextKey::RowNumbersAndColumnHeadersStayFixed,
            Self::CellValue => TextKey::StringNumberBoolProgressAndIconUse,
            Self::Header => TextKey::CustomHeadersCombineAStarAndLabel,
            Self::Selection => TextKey::ClickAColumnHeaderRowNumberOr,
            Self::Layout => TextKey::NameStays180PxWideNumberAndBool,
            Self::Virtualization => TextKey::Text2000Rows40ColumnsScrollHorizontally,
            Self::Disabled => TextKey::InitiallyDisabledEnableToSelectAndNavigate,
            Self::Shared => TextKey::TableSharedDescription,
        }
    }
}

impl Plugin for TableDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            app.register_widgetry_table::<DemoRow>();
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(
                |value: &TableLabel| {
                    bsn_list! { @{value.scene()} bevy_widgetry::text::WidgetryText }
                },
            ))?;
            app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(
                |value: &TableLabel| {
                    bsn_list! { @{value.scene()} bevy_widgetry::text::WidgetryText }
                },
            ))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
                bsn_list! {Text({ value.clone() }) bevy_widgetry::text::WidgetryText}
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &u32| {
                bsn_list! {Text({ format!("{value}") }) bevy_widgetry::text::WidgetryText}
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &bool| {
                bsn_list!{Text({ if *value { "true" } else { "false" } }) bevy_widgetry::text::WidgetryText}
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &ProgressValue| {
                let progress_percent = (value.0 * 100.0).clamp(0.0, 100.0);
                bsn_list!{Node {width: percent(100), align_items: AlignItems::Center, column_gap: px(8)} Children [
                    Node {width: px(90), height: px(10), overflow: Overflow::clip()} BackgroundColor(Color::srgb(0.24, 0.27, 0.32)) Children [
                        Node {width: percent(progress_percent), height: percent(100)} BackgroundColor(Color::srgb(0.22, 0.62, 0.90))
                    ]--
                    Text({format!("{progress_percent:.0}%")}) bevy_widgetry::text::WidgetryText
                ]}
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|_: &StarValue| bsn_list!{
                @WidgetryIcon { @path: {GalleryIcon::ButtonStar.path()}, @max_size: {Some(UVec2::new(18, 18))} }
                Node {width: px(18), height: px(18)}
            }))?;
            app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(
                |value: &String| bsn_list!{Text({ value.clone() }) bevy_widgetry::text::WidgetryText},
            ))?;
            app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &CustomHeader| bsn_list!{
                Node {align_items: AlignItems::Center, column_gap: px(8)} Children [
                    @WidgetryIcon { @path: {GalleryIcon::ButtonStar.path()}, @max_size: {Some(UVec2::new(16, 16))} } Node {width: px(16), height: px(16)}--
                    @{value.0.scene()} bevy_widgetry::text::WidgetryText
                ]
            }))?;
            app.register_type::<TableDemoSources>()
                .add_systems(OnEnter(GalleryPage::Table), enter)
                .add_systems(OnExit(GalleryPage::Table), exit)
                .add_observer(on_table_event)
                .add_systems(
                    PostUpdate,
                    update_status
                        .after(UiSystems::Layout)
                        .run_if(in_state(GalleryPage::Table)),
                );
            Ok(())
        })();
        if let Err(error) = result {
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(error) });
        }
    }
}

fn enter(world: &mut World) -> Result {
    let sources = sources(world)?;
    let content = scene(sources.clone());
    world.insert_resource(sources);
    if let Err(error) = mount_page(world, GalleryPage::Table, bsn_list! { @content }) {
        release_sources(world);
        return Err(error);
    }
    Ok(())
}

fn release_sources(world: &mut World) {
    if let Some(sources) = world
        .get_resource::<TableDemoSources>()
        .map(|sources| sources.sources)
    {
        for source in sources {
            world.despawn(source);
        }
        world.remove_resource::<TableDemoSources>();
    }
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Table)?;
    release_sources(world);
    Ok(())
}
