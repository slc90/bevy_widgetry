use crate::assets::GalleryIcon;
mod copy;
mod editing;
use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::TableKey;
use crate::i18n::{CommonKey, NavigationKey};
use crate::i18n::{Language, TextKey, localized, localized_args, tr, tr_args};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::{InteractionDisabled, ScrollPosition, UiSystems};
use bevy::ui_widgets::Activate;
use bevy::window::RequestRedraw;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::list_view::{WidgetryListModel, WidgetryListViewRenderer};
use bevy_widgetry::table::{
    WidgetryTable, WidgetryTableAppExt, WidgetryTableBody, WidgetryTableCell,
    WidgetryTableCellRenderer, WidgetryTableCellValue, WidgetryTableColumn, WidgetryTableColumnId,
    WidgetryTableColumnReordering, WidgetryTableColumnWidth, WidgetryTableEvent,
    WidgetryTableEventKind, WidgetryTableHeaderActions, WidgetryTableHeaderRenderer,
    WidgetryTableHeaderValue, WidgetryTableLayout, WidgetryTableModel, WidgetryTableSelection,
    WidgetryTableSort, WidgetryTableSortDirection, WidgetryTableState, WidgetryTableStyle,
    WidgetryTableViewState, WidgetryTableViewUpdate,
};
use bevy_widgetry::table::{
    WidgetryTableCellEditor, WidgetryTableEditRequested, WidgetryTableEditResult,
    WidgetryTableEditState, WidgetryTableEditing, WidgetryTableEditorAppExt,
    WidgetryTableEditorMode, WidgetryTableEditorRegistry,
};
use bevy_widgetry::text_field::WidgetryTextField;
use std::collections::HashSet;

pub(crate) struct TableDemoPlugin;

pub(crate) fn color_examples(world: &World) -> impl Scene + use<> {
    let sources = world.resource::<TableDemoSources>();
    let source = sources.sources[0];
    let layout = sources.layouts[0].clone();
    let view = move || bsn! { @WidgetryTable::<DemoRow> { @source: source, @layout: {layout.clone()} } Node { width: percent(100), height: px(160) } };
    crate::color_showcase::pair(
        TextKey::Navigation(NavigationKey::Table),
        TextKey::Table(TableKey::OnlyCellSelectedBackground0F766EMovePointer),
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
    #[reflect(ignore)]
    header_actions: [WidgetryTableHeaderActions; 8],
    next_shared_row: u32,
    next_column: usize,
}

struct DemoRow {
    notes: String,
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
    fn localization(&self) -> (TextKey, Vec<(&'static str, String)>) {
        match self {
            Self::Key(key) => (*key, vec![]),
            Self::Item(index) => (
                TextKey::Table(TableKey::TableItem),
                vec![("index", format!("{index:02}"))],
            ),
            Self::Updated(edits) => (
                TextKey::Table(TableKey::TableUpdated),
                vec![("edits", edits.to_string())],
            ),
            Self::Column(index) => (
                TextKey::Table(TableKey::TableColumn),
                vec![("index", format!("{index:02}"))],
            ),
        }
    }

    fn text(&self, language: Language) -> String {
        let (key, args) = self.localization();
        tr_args(language, key, &args)
    }

    fn scene(&self) -> impl Scene + use<> {
        let (key, args) = self.localization();
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

#[derive(Component)]
struct ProjectionDemo {
    enabled: bool,
    applied: Option<bevy::ecs::change_detection::Tick>,
}

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
    ToggleColumnReordering,
    ResetColumnOrder,
    InsertColumn,
    RemoveColumn,
    MoveColumn,
    ReplaceModel,
    CycleFrozen,
    CopySelection,
    ToggleCopyFormatter,
    ToggleEditing,
    ToggleProjection,
    SelectFirst,
    ClearSelection,
    SelectAll,
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
                @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(demo.title()) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
                template(move |_| Ok(DemoNav(demo)))
                template(move |_| Ok(Name::new(format!("TableDemoNav{demo:?}"))))
                on(change_demo)

            }
        })
        .collect::<Vec<_>>();
    let panels = demos
        .into_iter()
        .enumerate()
        .map(|(index, demo)| -> Box<dyn SceneList> {
            let source = sources.sources[index];
            let layout = sources.layouts[index].clone();
            let actions = sources.header_actions[index].clone();
            if demo == Demo::Shared {
                Box::new(bsn_list! { @shared_panel(source, layout, actions) })
            } else {
                Box::new(bsn_list! { @panel(source, layout, demo, actions) })
            }
        })
        .collect::<Vec<_>>();
    bsn! {
        template(|_| Ok(TableDemo))
        Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, row_gap: px(16) }
        Children [
            @localized(TextKey::Navigation(NavigationKey::Table)) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(28.0) }--
            Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap } Children [{buttons}]--
            Node { flex_grow: 1.0, min_height: px(0), min_width: px(0) } Children [{panels}]
        ]
    }
}

fn panel(
    source: Entity,
    layout: WidgetryTableLayout,
    demo: Demo,
    actions: WidgetryTableHeaderActions,
) -> impl Scene {
    let controls = [
        (TextKey::Table(TableKey::ToggleColumnReordering),Action::ToggleColumnReordering),
        (TextKey::Table(TableKey::ResetColumnOrder),Action::ResetColumnOrder),
        (TextKey::Table(TableKey::CycleFrozen), Action::CycleFrozen),
        (TextKey::Table(TableKey::CopySelection), Action::CopySelection),
        (TextKey::Table(TableKey::ToggleCopyFormatter), Action::ToggleCopyFormatter),
        (TextKey::Table(TableKey::ToggleEditing), Action::ToggleEditing),
        (
            TextKey::Table(TableKey::SelectFirstApi),
            Action::SelectFirst,
        ),
        (
            TextKey::Table(TableKey::ClearSelectionApi),
            Action::ClearSelection,
        ),
        (TextKey::Table(TableKey::SelectAllApi), Action::SelectAll),
        (TextKey::Table(TableKey::UpdateFirstRow), Action::UpdateRow),
        (
            TextKey::Table(TableKey::ReplaceFirstHeader),
            Action::ReplaceHeader,
        ),
        (
            TextKey::Common(CommonKey::EnableDisable),
            Action::ToggleDisabled,
        ),
        (
            TextKey::Common(CommonKey::ResizeViewport),
            Action::ResizeViewport,
        ),
    ]
    .into_iter()
    .map(|(label, kind)| {
        bsn! {
            @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(label) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
            template(move |_| Ok(DemoAction { source, view: DemoTable(demo, 0), kind }))
            template(move |_| Ok(Name::new(format!("Table{demo:?}{kind:?}"))))
            on(operate)

        }
    })
    .collect::<Vec<_>>();
    let table: Box<dyn SceneList> = Box::new(bsn_list! {
        @WidgetryTable::<DemoRow> { @source: source, @layout: {layout.clone()}, @header_actions: {actions.clone()}, @editing: {WidgetryTableEditing::new(move |_,_,_| demo != Demo::Virtualization)} }
        template(move |_| Ok(Name::new(format!("TableView{demo:?}"))))
        template(|_| Ok(ProjectionDemo { enabled: false, applied: None }))
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
            @localized(TextKey::Table(TableKey::SortFilterDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::RangeSelectionDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::EditingDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::FrozenDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::ColumnReorderingDescription)) bevy_widgetry::text::WidgetryText--
            @localized(demo.description()) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(16.0) }--
            template(move |_| Ok(DemoStatus { source, view: DemoTable(demo, 0) })) template(move |_| Ok(Name::new(format!("TableStatus{demo:?}")))) Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(14.0) }--
            {table}--
            Node {column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap} Children [{controls}]
            -- @copy::feedback(DemoTable(demo,0))
        ]
    }
}

fn shared_panel(
    source: Entity,
    layout: WidgetryTableLayout,
    actions: WidgetryTableHeaderActions,
) -> impl Scene {
    let other_actions = actions.clone();
    let mut compact = layout.clone();
    compact.row_height = 28.0;
    compact.default_column_width = WidgetryTableColumnWidth::Fixed(120.0);
    compact.frozen_columns = 1;
    bsn! {
        template(|_| Ok(DemoPanel(Demo::Shared)))
        Node { width: percent(100), flex_direction: FlexDirection::Column, row_gap: px(12), min_width: px(0), display: Display::None }
        Children [
            @localized(TextKey::Table(TableKey::SortFilterDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::RangeSelectionDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::TableSharedDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::ColumnReorderingDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::EditingDescription)) bevy_widgetry::text::WidgetryText--
            @localized(TextKey::Table(TableKey::FrozenDescription)) bevy_widgetry::text::WidgetryText--
            Node { width: percent(100), column_gap: px(16), align_items: AlignItems::Start } Children [
                @shared_view(source, layout, 0, actions)--
                @shared_view(source, compact, 1, other_actions)
            ]
        ]
    }
}

fn shared_view(
    source: Entity,
    layout: WidgetryTableLayout,
    side: usize,
    actions: WidgetryTableHeaderActions,
) -> impl Scene {
    let name = if side == 0 { "SharedA" } else { "SharedB" };
    let view = DemoTable(Demo::Shared, side);
    let controls = [
        (TextKey::Table(TableKey::ToggleColumnReordering),Action::ToggleColumnReordering),
        (TextKey::Table(TableKey::ResetColumnOrder),Action::ResetColumnOrder),
        (TextKey::Table(TableKey::InsertColumn),Action::InsertColumn),
        (TextKey::Table(TableKey::RemoveColumn),Action::RemoveColumn),
        (TextKey::Table(TableKey::MoveColumn),Action::MoveColumn),
        (TextKey::Table(TableKey::ReplaceModel),Action::ReplaceModel),
        (TextKey::Table(TableKey::CycleFrozen), Action::CycleFrozen),
        (TextKey::Table(TableKey::CopySelection), Action::CopySelection),
        (TextKey::Table(TableKey::ToggleCopyFormatter), Action::ToggleCopyFormatter),
        (TextKey::Table(TableKey::ToggleEditing), Action::ToggleEditing),
        (TextKey::Table(TableKey::ToggleProjection), Action::ToggleProjection),
        (
            TextKey::Table(TableKey::SelectFirstApi),
            Action::SelectFirst,
        ),
        (
            TextKey::Table(TableKey::ClearSelectionApi),
            Action::ClearSelection,
        ),
        (TextKey::Table(TableKey::SelectAllApi), Action::SelectAll),
        (TextKey::Table(TableKey::UpdateFirstRow), Action::UpdateRow),
        (TextKey::Table(TableKey::TableInsertRow), Action::InsertRow),
        (TextKey::Common(CommonKey::RemoveFirst), Action::RemoveRow),
        (TextKey::Common(CommonKey::MoveFirstLast), Action::MoveRow),
        (
            TextKey::Common(CommonKey::EnableDisable),
            Action::ToggleDisabled,
        ),
    ]
    .into_iter()
    .map(move |(label, kind)| {
        bsn! {
            @WidgetryButton { @children: {Some(Box::new(bsn_list! {@localized(label) bevy_widgetry::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
            template(move |_| Ok(DemoAction { source, view, kind }))
            Name({format!("Table{name}{kind:?}")}) on(operate)

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
            @localized(if side == 0 {TextKey::Table(TableKey::TableSharedA)} else {TextKey::Table(TableKey::TableSharedB)}) bevy_widgetry::text::WidgetryText--
            template(move |_| Ok(DemoStatus { source, view })) Name({format!("TableStatus{name}")}) Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(13.0) }--
            @WidgetryTable::<DemoRow> { @source: source, @layout: {layout.clone()}, @style: {style.clone()}, @header_actions: {actions.clone()}, @editing: {WidgetryTableEditing::new(|_,_,_| true)} }
            template(move |_| Ok(view)) template(|_| Ok(DemoEvents::default()))
            Name({format!("TableView{name}")}) TabIndex(-1)
                template(move |_| Ok(ProjectionDemo { enabled: side == 1, applied: None }))
            Node { width: percent(100), height: px(240), flex_shrink: 0.0 }--
            Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap } Children [{controls}]
            -- @copy::feedback(view)
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
            Action::ToggleColumnReordering => {
                let mut config = world
                    .get_mut::<WidgetryTableColumnReordering>(root)
                    .ok_or_else(|| BevyError::error("Table column reordering config missing"))?;
                config.enabled = !config.enabled;
                info!(?root, enabled = config.enabled, "切换 Table 列拖拽");
            }
            Action::ResetColumnOrder => {
                let changed = WidgetryTable::<DemoRow>::reset_column_order(world, root)?;
                info!(?root, changed, "重置 Table View 列顺序");
            }
            Action::InsertColumn => {
                let index = world.resource::<TableDemoSources>().next_column;
                let mut model = world
                    .get_mut::<WidgetryTableModel<DemoRow>>(action.source)
                    .ok_or_else(|| BevyError::error("Table Model missing"))?;
                let at = model.column_count().min(1);
                let column = model.insert_column(at, column(index, false, false))?;
                world.resource_mut::<TableDemoSources>().next_column += 1;
                for (view, mut actions) in world
                    .query::<(&WidgetryTable<DemoRow>, &mut WidgetryTableHeaderActions)>()
                    .iter_mut(world)
                {
                    if view.source() == action.source {
                        actions.sortable.insert(column);
                        actions.filterable.insert(column);
                    }
                }
                info!(?root, ?column, "在 Model 第二列位置插入列");
            }
            Action::RemoveColumn | Action::MoveColumn => {
                let mut model = world
                    .get_mut::<WidgetryTableModel<DemoRow>>(action.source)
                    .ok_or_else(|| BevyError::error("Table Model missing"))?;
                if matches!(action.kind, Action::RemoveColumn) {
                    model.remove_column(0);
                    info!(?root, "删除 Model 首列");
                } else {
                    let last = model.column_count().saturating_sub(1);
                    let changed = model.move_column(0, last);
                    info!(?root, changed, "将 Model 首列移到末尾");
                }
            }
            Action::ReplaceModel => {
                let model = make_model(7)?;
                let columns = (0..model.column_count())
                    .filter_map(|i| model.column_id(i))
                    .collect::<HashSet<_>>();
                let roots = world
                    .query::<(Entity, &WidgetryTable<DemoRow>)>()
                    .iter(world)
                    .filter(|(_, view)| view.source() == action.source)
                    .map(|(root, _)| root)
                    .collect::<HashSet<_>>();
                world.entity_mut(action.source).insert(model);
                for root in roots {
                    world.entity_mut(root).insert(WidgetryTableHeaderActions {
                        sortable: columns.clone(),
                        filterable: columns.clone(),
                    });
                    if let Some(mut projection) = world.get_mut::<ProjectionDemo>(root) {
                        projection.applied = None;
                    }
                }
                info!(source=?action.source,"替换共享 Table Model 实例");
            }
            Action::CycleFrozen => {
                let mut layout = world
                    .get_mut::<WidgetryTableLayout>(root)
                    .ok_or_else(|| BevyError::error("Table Layout missing"))?;
                layout.frozen_columns = match layout.frozen_columns {
                    0 => 1,
                    1 => 2,
                    2 => 99,
                    _ => 0,
                };
                info!(?root, count = layout.frozen_columns, "切换 Table 冻结列数");
            }
            Action::CopySelection => copy::run(world, root, action.view),
            Action::ToggleCopyFormatter => copy::toggle(world),
            Action::ToggleEditing => {
                if let Some(mut config) = world.get_mut::<WidgetryTableEditing>(root) {
                    config.enabled = !config.enabled;
                }
                info!(?root, "切换 Table 编辑模式");
            }
            Action::ToggleProjection => {
                let mut demo = world
                    .get_mut::<ProjectionDemo>(root)
                    .ok_or_else(|| BevyError::error("Gallery Table projection control missing"))?;
                demo.enabled = !demo.enabled;
                demo.applied = None;
                info!(?root, enabled = demo.enabled, "切换 Table 受控行投影");
            }
            Action::SelectFirst => {
                if let Some((row, column)) = world
                    .get::<WidgetryTableViewState>(root)
                    .and_then(|state| state.row_id(0).zip(state.column_id(0)))
                {
                    let changed = WidgetryTable::<DemoRow>::set_selection(
                        world,
                        root,
                        WidgetryTableSelection::Cell { row, column },
                    )?;
                    info!(?root, changed, "程序化设置 Table selection");
                }
            }
            Action::SelectAll => {
                let changed = WidgetryTable::<DemoRow>::set_selection(
                    world,
                    root,
                    WidgetryTableSelection::All,
                )?;
                info!(?root, changed, "程序化全选当前 Table View");
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
                            TextKey::Table(TableKey::TableUpdatedHeader),
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
                                notes: "Notes".to_owned(),
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

fn refresh_shared_projection(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, (With<WidgetryTable<DemoRow>>, With<ProjectionDemo>)>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let source = world
            .get::<WidgetryTable<DemoRow>>(root)
            .ok_or_else(|| BevyError::error("Gallery Table View missing"))?
            .source();
        let entity = world.get_entity(source).map_err(BevyError::error)?;
        let model = entity
            .get_ref::<WidgetryTableModel<DemoRow>>()
            .ok_or_else(|| BevyError::error("Gallery Table Model missing"))?;
        let tick = model.last_changed();
        let demo = world
            .get::<ProjectionDemo>(root)
            .ok_or_else(|| BevyError::error("Gallery Table projection settings missing"))?;
        if demo.applied == Some(tick) {
            continue;
        }
        let state = world
            .get::<WidgetryTableViewState>(root)
            .ok_or_else(|| BevyError::error("Gallery Table projection missing"))?;
        let (sort, filters) = if demo.applied.is_none() {
            let column = model.column_id(0);
            if demo.enabled {
                (
                    column.map(|column| WidgetryTableSort {
                        column,
                        direction: WidgetryTableSortDirection::Descending,
                    }),
                    column.into_iter().collect(),
                )
            } else {
                (None, HashSet::new())
            }
        } else {
            (
                state
                    .sort()
                    .filter(|sort| model.column_index(sort.column).is_some()),
                state
                    .filters()
                    .iter()
                    .copied()
                    .filter(|column| model.column_index(*column).is_some())
                    .collect(),
            )
        };
        project(world, root, sort, filters)?;
        if let Some(mut demo) = world.get_mut::<ProjectionDemo>(root) {
            demo.applied = Some(tick);
        }
    }
    Ok(())
}

fn project(
    world: &mut World,
    root: Entity,
    sort: Option<WidgetryTableSort>,
    filters: HashSet<WidgetryTableColumnId>,
) -> Result {
    if sort.is_none() && filters.is_empty() {
        WidgetryTable::<DemoRow>::reset_view(world, root)?;
        return Ok(());
    }
    let source = world
        .get::<WidgetryTable<DemoRow>>(root)
        .ok_or_else(|| BevyError::error("Gallery Table View missing"))?
        .source();
    let model = world
        .get::<WidgetryTableModel<DemoRow>>(source)
        .ok_or_else(|| BevyError::error("Gallery Table Model missing"))?;
    let mut rows = (0..model.row_count())
        .filter(|&index| {
            let Some(row) = model.row(index) else {
                return false;
            };
            filters.iter().all(|column| {
                let Some(value) = model.row_id(index).and_then(|row| model.cell(row, *column))
                else {
                    return false;
                };
                if let Some(value) = value.downcast_ref::<u32>() {
                    value % 3 == 0
                } else if let Some(value) = value.downcast_ref::<bool>() {
                    *value
                } else if let Some(value) = value.downcast_ref::<ProgressValue>() {
                    value.0 >= 0.5
                } else if value.downcast_ref::<TableLabel>().is_some() {
                    row.index % 2 == 0
                } else {
                    row.index % 5 == 0
                }
            })
        })
        .collect::<Vec<_>>();
    if let Some(sort) = sort {
        if model.column_index(sort.column).is_none() {
            return Err(BevyError::error("Gallery sort column missing"));
        }
        rows.sort_by(|&a, &b| {
            let values = model
                .row_id(a)
                .zip(model.row_id(b))
                .and_then(|(a, b)| model.cell(a, sort.column).zip(model.cell(b, sort.column)));
            let order = values
                .map(|(a_value, b_value)| {
                    if let Some((a, b)) = a_value
                        .downcast_ref::<u32>()
                        .zip(b_value.downcast_ref::<u32>())
                    {
                        a.cmp(b)
                    } else if let Some((a, b)) = a_value
                        .downcast_ref::<bool>()
                        .zip(b_value.downcast_ref::<bool>())
                    {
                        a.cmp(b)
                    } else if let Some((a, b)) = a_value
                        .downcast_ref::<ProgressValue>()
                        .zip(b_value.downcast_ref::<ProgressValue>())
                    {
                        a.0.total_cmp(&b.0)
                    } else {
                        model
                            .row(a)
                            .zip(model.row(b))
                            .map_or(std::cmp::Ordering::Equal, |(a, b)| a.index.cmp(&b.index))
                    }
                })
                .unwrap_or(std::cmp::Ordering::Equal);
            if sort.direction == WidgetryTableSortDirection::Descending {
                order.reverse()
            } else {
                order
            }
        });
    }
    let rows = rows
        .into_iter()
        .filter_map(|index| model.row_id(index))
        .collect();
    WidgetryTable::<DemoRow>::update_view(
        world,
        root,
        WidgetryTableViewUpdate {
            rows,
            sort,
            filters,
        },
    )?;
    Ok(())
}

fn on_header_request(
    event: On<WidgetryTableEvent>,
    demos: Query<(), With<ProjectionDemo>>,
    mut commands: Commands,
) {
    if !demos.contains(event.entity)
        || !matches!(
            event.kind,
            WidgetryTableEventKind::SortRequested { .. }
                | WidgetryTableEventKind::FilterRequested(_)
        )
    {
        return;
    }
    let root = event.entity;
    let kind = event.kind;
    commands.queue(move |world: &mut World| -> Result {
        let Some(state) = world.get::<WidgetryTableViewState>(root) else {
            return Ok(());
        };
        let mut sort = state.sort();
        let mut filters = state.filters().clone();
        match kind {
            WidgetryTableEventKind::SortRequested { column, direction } => {
                sort = direction.map(|direction| WidgetryTableSort { column, direction })
            }
            WidgetryTableEventKind::FilterRequested(column) => {
                if !filters.remove(&column) {
                    filters.insert(column);
                }
            }
            _ => return Ok(()),
        }
        project(world, root, sort, filters)?;
        info!(?root, ?kind, "应用 Table 排序筛选请求");
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
        | WidgetryTableEventKind::SelectionCleared
        | WidgetryTableEventKind::RangeSelected(_) => {
            counts.selection += 1;
            info!(view = ?event.entity, selection = ?event.kind, "Table selection 变化");
        }
        WidgetryTableEventKind::SortRequested { .. }
        | WidgetryTableEventKind::FilterRequested(_) => {}
        WidgetryTableEventKind::ColumnOrderChanged => {
            info!(view=?event.entity,"Table View 列顺序变化")
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
        let projection = world
            .get::<WidgetryTableViewState>(root)
            .ok_or_else(|| BevyError::error("Gallery Table row projection missing"))?;
        let indices = cells
            .iter()
            .filter(|(owner, _)| *owner == root)
            .filter_map(|(_, cell)| {
                projection
                    .display_index(cell.row)
                    .zip(projection.column_display_index(cell.column))
            })
            .collect::<Vec<_>>();
        let cursor = state.focused_cell().and_then(|cell| {
            projection
                .display_index(cell.row)
                .zip(projection.column_display_index(cell.column))
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
        let kind = tr(
            language,
            TextKey::Table(match state.selection() {
                WidgetryTableSelection::None => TableKey::SelectionEmpty,
                WidgetryTableSelection::Row(_) | WidgetryTableSelection::Rows { .. } => {
                    TableKey::SelectionRows
                }
                WidgetryTableSelection::Column(_) | WidgetryTableSelection::Columns { .. } => {
                    TableKey::SelectionColumns
                }
                WidgetryTableSelection::All => TableKey::SelectionAll,
                _ => TableKey::SelectionCells,
            }),
        );
        let mut label = tr_args(
            language,
            TextKey::Table(TableKey::TableStatus),
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
                ("selection", kind.into()),
                ("cursor", format!("{cursor:?}")),
                ("scroll", format!("{:.1}, {:.1}", scroll.x, scroll.y)),
                (
                    "state",
                    tr(
                        language,
                        if world.get::<InteractionDisabled>(root).is_some() {
                            TextKey::Common(CommonKey::Disabled)
                        } else {
                            TextKey::Common(CommonKey::Enabled)
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
        let sample = projection
            .ordered_rows()
            .iter()
            .take(8)
            .filter_map(|row| model.row_index(*row).and_then(|index| model.row(index)))
            .map(|row| row.index.to_string())
            .collect::<Vec<_>>()
            .join(",");
        label.push_str(&tr_args(
            language,
            TextKey::Table(TableKey::ProjectionStatus),
            &[("count", projection.len().to_string()), ("order", sample)],
        ));
        let range = WidgetryTable::<DemoRow>::selection_range(world, root)?;
        label.push_str(&tr_args(
            language,
            TextKey::Table(TableKey::SelectionRangeStatus),
            &[
                ("kind", kind.into()),
                (
                    "rows",
                    range.map_or_else(
                        || "-".into(),
                        |range| format!("{}..{}", range.rows().start + 1, range.rows().end),
                    ),
                ),
                (
                    "columns",
                    range.map_or_else(
                        || "-".into(),
                        |range| format!("{}..{}", range.columns().start + 1, range.columns().end),
                    ),
                ),
            ],
        ));
        label.push_str(&tr_args(
            language,
            TextKey::Table(TableKey::SortFilterStatus),
            &[
                ("sort", format!("{:?}", projection.sort())),
                ("filters", projection.filters().len().to_string()),
            ],
        ));
        let frozen = world
            .get::<WidgetryTableLayout>(root)
            .map_or(0, |layout| layout.frozen_columns);
        label.push_str(&tr_args(
            language,
            TextKey::Table(TableKey::FrozenStatus),
            &[
                ("configured", frozen.to_string()),
                ("effective", frozen.min(model.column_count()).to_string()),
            ],
        ));
        let name = |column| {
            model.header(column).map_or_else(
                || "?".into(),
                |value| {
                    value
                        .downcast_ref::<TableLabel>()
                        .or_else(|| value.downcast_ref::<CustomHeader>().map(|h| &h.0))
                        .map_or_else(|| format!("{column:?}"), |label| label.text(language))
                },
            )
        };
        let model_order = (0..model.column_count())
            .take(8)
            .filter_map(|i| model.column_id(i))
            .map(name)
            .collect::<Vec<_>>()
            .join(", ");
        let view_order = projection
            .ordered_columns()
            .iter()
            .copied()
            .take(8)
            .map(name)
            .collect::<Vec<_>>()
            .join(", ");
        let enabled = world
            .get::<WidgetryTableColumnReordering>(root)
            .is_some_and(|config| config.enabled);
        label.push_str(&tr_args(
            language,
            TextKey::Table(TableKey::ColumnOrderStatus),
            &[
                ("model", model_order),
                ("view", view_order),
                (
                    "state",
                    tr(
                        language,
                        TextKey::Common(if enabled {
                            CommonKey::Enabled
                        } else {
                            CommonKey::Disabled
                        }),
                    )
                    .into(),
                ),
            ],
        ));
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
    } else if index == 5 || index == 6 {
        TableLabel::Key(TextKey::Table(if index == 5 {
            TableKey::SingleLineNotes
        } else {
            TableKey::MultiLineNotes
        }))
    } else {
        TableLabel::Key(
            [
                TextKey::Table(TableKey::TableName),
                TextKey::Table(TableKey::TableNumber),
                TextKey::Table(TableKey::TableBool),
                TextKey::Table(TableKey::TableProgress),
                TextKey::Common(CommonKey::Icon),
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
        if *index == 5 || *index == 6 {
            return WidgetryTableCellValue::new(row.notes.clone());
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

fn make_model(index: usize) -> Result<WidgetryTableModel<DemoRow>, BevyError> {
    let mut model = WidgetryTableModel::default();
    let virtualized = index == 5;
    for row in 0..if virtualized {
        2000
    } else if index == 7 {
        64
    } else {
        24
    } {
        model.push_row(DemoRow {
            notes: if row == 0 {
                "Tab\tquote \"notes\"\nsecond line".to_owned()
            } else {
                format!("Notes {row}")
            },
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
    } else if index == 7 || index == 1 {
        8
    } else if index == 0 || index == 3 || index == 4 {
        3
    } else {
        5
    };
    for col in 0..columns {
        model.push_column(column(col, index == 2, virtualized))?;
    }
    Ok(model)
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
        *model = make_model(index)?;
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
    let header_actions = std::array::from_fn(|index| {
        let model = &models[index];
        let columns = (0..model.column_count())
            .filter_map(|index| model.column_id(index))
            .collect::<HashSet<_>>();
        WidgetryTableHeaderActions {
            sortable: columns.clone(),
            filterable: columns,
        }
    });
    let sources = models.map(|model| world.spawn(model).id());
    editing::register_columns(world, &sources);
    copy::register_columns(world, &sources);
    Ok(TableDemoSources {
        sources,
        layouts,
        header_actions,
        next_shared_row: 64,
        next_column: 40,
    })
}

impl Demo {
    fn title(self) -> TextKey {
        match self {
            Self::Basic => TextKey::Common(CommonKey::Basic),
            Self::CellValue => TextKey::Table(TableKey::Cellvalue),
            Self::Header => TextKey::Table(TableKey::Header),
            Self::Selection => TextKey::Table(TableKey::Selection),
            Self::Layout => TextKey::Table(TableKey::ColumnLayout),
            Self::Virtualization => TextKey::Table(TableKey::Virtualization),
            Self::Disabled => TextKey::Common(CommonKey::Disabled),
            Self::Shared => TextKey::Table(TableKey::TableShared),
        }
    }

    fn description(self) -> TextKey {
        match self {
            Self::Basic => TextKey::Table(TableKey::RowNumbersAndColumnHeadersStayFixed),
            Self::CellValue => TextKey::Table(TableKey::StringNumberBoolProgressAndIconUse),
            Self::Header => TextKey::Table(TableKey::CustomHeadersCombineAStarAndLabel),
            Self::Selection => TextKey::Table(TableKey::ClickAColumnHeaderRowNumberOr),
            Self::Layout => TextKey::Table(TableKey::NameStays180PxWideNumberAndBool),
            Self::Virtualization => {
                TextKey::Table(TableKey::Text2000Rows40ColumnsScrollHorizontally)
            }
            Self::Disabled => TextKey::Table(TableKey::InitiallyDisabledEnableToSelectAndNavigate),
            Self::Shared => TextKey::Table(TableKey::TableSharedDescription),
        }
    }
}

impl Plugin for TableDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            app.register_widgetry_table::<DemoRow>();
            editing::install(app)?;
            copy::install(app);
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
            app.add_systems(
                PostUpdate,
                refresh_shared_projection
                    .before(bevy_widgetry::table::WidgetryTableSystems::Projection)
                    .run_if(in_state(GalleryPage::Table)),
            );
            app.register_type::<TableDemoSources>()
                .add_systems(OnEnter(GalleryPage::Table), enter)
                .add_systems(OnExit(GalleryPage::Table), exit)
                .add_observer(on_table_event)
                .add_observer(on_header_request)
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
            world
                .resource_mut::<WidgetryTableEditorRegistry>()
                .remove_source(source);
            world
                .resource_mut::<bevy_widgetry::table::WidgetryTableCopyFormatterRegistry>()
                .remove_source(source);
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
