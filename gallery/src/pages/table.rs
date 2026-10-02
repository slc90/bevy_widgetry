use crate::assets::GalleryIcon;
use bevy::app::Propagate;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::{InteractionDisabled, ScrollPosition, UiSystems};
use bevy::ui_widgets::Activate;
use bevy::window::RequestRedraw;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::icon::WidgetryIcon;
use bevy_widgetry::style::{ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry::table::{
    WidgetryTable, WidgetryTableAppExt, WidgetryTableBody, WidgetryTableCell,
    WidgetryTableCellRenderer, WidgetryTableCellValue, WidgetryTableColumn,
    WidgetryTableColumnWidth, WidgetryTableEvent, WidgetryTableEventKind,
    WidgetryTableHeaderRenderer, WidgetryTableHeaderValue, WidgetryTableLayout, WidgetryTableModel,
    WidgetryTableSelection, WidgetryTableState,
};

/// Gallery 自有业务值和 renderer，展示库的公共二维能力。
pub(crate) struct TableDemoPlugin;

/// 七个独立 Model 与各 View 的初始 layout；页面切换保留运行期 Component。
#[derive(Resource, Clone)]
pub(crate) struct TableDemoSources {
    /// 七类示例按导航顺序引用各自业务 Model。
    pub(crate) sources: [Entity; 7],
    /// 只供第一次 BSN 展开，运行期修改留在 View Component。
    layouts: [WidgetryTableLayout; 7],
}

/// Row 保存业务数据，Column schema 投影出不同 value type。
struct DemoRow {
    /// 原始业务编号，scroll 后仍可从 Content 确认当前数据。
    index: u32,
    /// String Column 的当前业务内容。
    label: String,
    /// Number Column 的独立业务值。
    number: u32,
    /// Bool Column 的值，不代表 Table interaction disabled。
    enabled: bool,
    /// Progress Column 的归一化比例。
    progress: f32,
    /// 内容更新次数，用于确认 offscreen 数据再次投影时的版本。
    edits: u32,
}

/// Progress renderer 接收归一化业务值，不依赖 physical Cell 生命周期。
#[derive(Reflect)]
struct ProgressValue(f32);

/// Icon renderer 的业务语义，asset 从 Gallery 的语义标识访问。
#[derive(Reflect)]
struct StarValue;

/// 自定义 Header 将 icon 与 label 组合，独立于 Cell registry。
#[derive(Reflect)]
struct CustomHeader(String);

/// 页面的 foreground propagation 边界。
#[derive(Component)]
struct TableDemo;

/// 七类 Demo 的身份用于导航与 panel 显隐。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Demo {
    Basic,
    CellValue,
    Header,
    Selection,
    Layout,
    Virtualization,
    Disabled,
}

/// 导航按钮只控制 Demo panel，不保存重复的当前页 state。
#[derive(Component)]
struct DemoNav(Demo);

/// 同一个 panel Node.display 是显示状态的唯一来源。
#[derive(Component)]
struct DemoPanel(Demo);

/// 仅当前显示的 Table 参与页内 Tab navigation。
#[derive(Component)]
struct DemoTable(Demo);

/// status 从 source、公开 state 和 hierarchy 计算，统计离散 event。
#[derive(Component, Default)]
struct DemoEvents {
    /// UI 与程序 selection 实际变化递增，包括显式清空。
    selection: usize,
    /// 已开始的 Column gesture 次数。
    resize_start: usize,
    /// 实际 width 变化通知次数。
    resize_changes: usize,
    /// 正常结束的 Column gesture 次数。
    resize_end: usize,
    /// 取消/中断的 Column gesture 次数，保留最后 width。
    resize_cancel: usize,
}

/// status 绑定同源 View，绝不以 physical Cell index 保存 selection。
#[derive(Component)]
struct DemoStatus(Entity);

/// 控制按钮携带明确 source 与业务操作。
#[derive(Component, Clone, Copy)]
struct DemoAction {
    /// 操作所属示例的独立 Model。
    source: Entity,
    /// 当前按钮的离散业务语义。
    kind: Action,
}

/// 只消费公开 API，不新增排序、编辑、多选或其他库行为。
#[derive(Clone, Copy, Debug)]
enum Action {
    SelectFirst,
    ClearSelection,
    UpdateRow,
    ReplaceHeader,
    ToggleDisabled,
    ResizeViewport,
}

/// 七类 Demo 共享页面空间；各 panel/source 常驻，切换不重建 Table。
pub(crate) fn scene(sources: TableDemoSources) -> impl Scene {
    let demos = [
        Demo::Basic,
        Demo::CellValue,
        Demo::Header,
        Demo::Selection,
        Demo::Layout,
        Demo::Virtualization,
        Demo::Disabled,
    ];
    let buttons = demos
        .into_iter()
        .map(|demo| {
            bsn! {
                @WidgetryButton
                template(move |_| Ok(DemoNav(demo)))
                template(move |_| Ok(Name::new(format!("TableDemoNav{demo:?}"))))
                on(change_demo)
                Children [Text({demo.title()})]
            }
        })
        .collect::<Vec<_>>();
    let panels = demos
        .into_iter()
        .enumerate()
        .map(|(index, demo)| panel(sources.sources[index], sources.layouts[index].clone(), demo))
        .collect::<Vec<_>>();
    bsn! {
        template(|_| Ok(TableDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node { width: percent(100), height: percent(100), flex_direction: FlexDirection::Column, row_gap: px(16) }
        Children [
            (Text("Table") TextFont { font_size: FontSize::Px(28.0) }),
            (Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap } Children [{buttons}]),
            (Node { flex_grow: 1.0, min_height: px(0), min_width: px(0) } Children [{panels}]),
        ]
    }
}

/// 每类示例都有有界 viewport、用户说明、state/status 与公开 API 演示按钮。
fn panel(source: Entity, layout: WidgetryTableLayout, demo: Demo) -> impl Scene {
    let controls = [
        ("Select first (API)", Action::SelectFirst),
        ("Clear selection (API)", Action::ClearSelection),
        ("Update first row", Action::UpdateRow),
        ("Replace first header", Action::ReplaceHeader),
        ("Enable / Disable", Action::ToggleDisabled),
        ("Resize viewport", Action::ResizeViewport),
    ]
    .into_iter()
    .map(|(label, kind)| {
        bsn! {
            @WidgetryButton
            template(move |_| Ok(DemoAction { source, kind }))
            template(move |_| Ok(Name::new(format!("Table{demo:?}{kind:?}"))))
            on(operate)
            Children [Text(label)]
        }
    })
    .collect::<Vec<_>>();
    let table: Box<dyn SceneList> = Box::new(bsn_list![(
        @WidgetryTable::<DemoRow> { @source: source, @layout: {layout.clone()} }
        template(move |_| Ok(Name::new(format!("TableView{demo:?}"))))
        template(|_| Ok(DemoEvents::default())) TabIndex({if demo == Demo::Basic {0} else {-1}})
        Node { width: percent(100), height: px(440), flex_shrink: 0.0 }
        template(move |context| {
            if demo == Demo::Disabled { context.entity.insert(InteractionDisabled); }
            Ok(DemoTable(demo))
        })
    )]);
    bsn! {
        template(move |_| Ok(DemoPanel(demo)))
        Node {
            width: percent(100), height: percent(100), flex_direction: FlexDirection::Column,
            row_gap: px(12), min_width: px(0),
            display: {if demo == Demo::Basic {Display::Flex} else {Display::None}},
        }
        Children [
            (Text({demo.description()}) TextFont { font_size: FontSize::Px(16.0) }),
            (template(move |_| Ok(DemoStatus(source))) template(move |_| Ok(Name::new(format!("TableStatus{demo:?}")))) Text("") TextFont { font_size: FontSize::Px(14.0) }),
            {table},
            (Node {column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap} Children [{controls}]),
        ]
    }
}

/// 真实 Activate 切换示例，日志记录离散展示语义。
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

/// 通过匹配 source 找唯一业务 View；失败记录 Gallery invariant 并返回 Error。
fn view(world: &mut World, source: Entity) -> Result<Entity, BevyError> {
    world
        .query::<(Entity, &WidgetryTable<DemoRow>)>()
        .iter(world)
        .find(|(_, table)| table.source() == source)
        .map(|(entity, _)| entity)
        .ok_or_else(|| {
            error!(?source, "Gallery Table 示例缺少 View");
            BevyError::error("Gallery Table View missing")
        })
}

/// 按实际按钮输入执行公开 API，成功后才记录操作结果；初始化不通知。
fn operate(event: On<Activate>, actions: Query<&DemoAction>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let root = view(world, action.source)?;
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
                        row.label = format!("Updated {}", row.edits);
                        row.number += 1;
                        row.enabled = !row.enabled;
                        row.progress = (row.progress + 0.15) % 1.0;
                        info!(?root, edits = row.edits, "更新 Table 首行内容");
                    }
                } else {
                    model.set_header(
                        0,
                        WidgetryTableHeaderValue::new(CustomHeader("Updated header".into())),
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

/// selection 记录已提交变化，resize 分别记录正常结束/取消，不记录连续 drag。
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

/// 沿公开 hierarchy 找 View，不引用库的私有 runtime 或 canvas cache。
fn owner(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryTable<DemoRow>>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

/// Layout 后读取当前 physical Cell 与 logical state，只更新可观察 status，不新增日志 polling。
fn update_status(world: &mut World) -> Result<(), BevyError> {
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
        .map(|(entity, status)| (entity, status.0))
        .collect::<Vec<_>>();
    if statuses.is_empty() {
        return Ok(());
    }
    let cells = world
        .query::<(Entity, &WidgetryTableCell)>()
        .iter(world)
        .filter_map(|(entity, cell)| owner(world, entity).map(|root| (root, *cell)))
        .collect::<Vec<_>>();
    for (entity, source) in statuses {
        let root = view(world, source)?;
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
        let label = format!(
            "Model: {} rows × {} columns | visible cells: {} | rows {:?}..{:?}, columns {:?}..{:?}\nselection: {:?} | cursor: {:?} | scroll: {:.1}, {:.1} | {} | events: select {}, resize start/change/end/cancel {}/{}/{}/{}",
            model.row_count(),
            model.column_count(),
            indices.len(),
            indices.iter().map(|pair| pair.0).min(),
            indices.iter().map(|pair| pair.0).max(),
            indices.iter().map(|pair| pair.1).min(),
            indices.iter().map(|pair| pair.1).max(),
            state.selection(),
            cursor,
            scroll.x,
            scroll.y,
            if world.get::<InteractionDisabled>(root).is_some() {
                "disabled"
            } else {
                "enabled"
            },
            events.selection,
            events.resize_start,
            events.resize_changes,
            events.resize_end,
            events.resize_cancel
        );
        if let Some(mut text) = world.get_mut::<Text>(entity)
            && text.0 != label
        {
            text.0 = label;
            // status 在 Layout 后读取真实范围，Reactive 宿主需再推进一次 Text 消费。
            world.write_message(RequestRedraw);
        }
    }
    Ok(())
}

/// 页面自有文字跟随 theme，Table shell 与内容继承由 Widgetry 维护。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<TableDemo>>,
) {
    for mut color in &mut roots {
        color.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

/// 独立的 Column schema 决定当前 Row 的实际异构 CellValue。
fn column(index: usize, custom: bool, virtualized: bool) -> WidgetryTableColumn<DemoRow> {
    let label = if virtualized {
        format!("Column {index:02}")
    } else {
        ["Name", "Number", "Bool", "Progress", "Icon"][index % 5].to_owned()
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

/// Gallery Model 留在独立 entity，width 初始化只发生一次。
fn sources(world: &mut World) -> Result<TableDemoSources, BevyError> {
    let mut sources = [Entity::PLACEHOLDER; 7];
    let mut layouts = std::array::from_fn(|_| {
        let mut layout = WidgetryTableLayout::default();
        layout.row_height = 34.0;
        layout.default_column_width = WidgetryTableColumnWidth::Fixed(180.0);
        layout
    });
    for index in 0..7 {
        let virtualized = index == 5;
        let mut model = WidgetryTableModel::default();
        for row in 0..if virtualized { 2000 } else { 24 } {
            model.push_row(DemoRow {
                index: row,
                label: format!("Item {row:02}"),
                number: row * 17 + 3,
                enabled: row % 2 == 0,
                progress: (row % 11) as f32 / 10.0,
                edits: 0,
            })?;
        }
        let columns = if virtualized {
            40
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
        sources[index] = world.spawn(model).id();
    }
    Ok(TableDemoSources { sources, layouts })
}

impl Demo {
    /// 导航使用原方案中的七类 Demo 名称。
    fn title(self) -> &'static str {
        match self {
            Self::Basic => "Basic",
            Self::CellValue => "CellValue",
            Self::Header => "Header",
            Self::Selection => "Selection",
            Self::Layout => "Column Layout",
            Self::Virtualization => "Virtualization",
            Self::Disabled => "Disabled",
        }
    }

    /// 说明当前可实际操作的行为，不暴露库内部实现。
    fn description(self) -> &'static str {
        match self {
            Self::Basic => {
                "Row numbers and column headers stay fixed on their own axis. Click any cell to select."
            }
            Self::CellValue => {
                "String, Number, Bool, Progress and Icon use distinct renderers. Update first row to refresh its content."
            }
            Self::Header => {
                "Custom headers combine a star and label. Header and Cell content use independent renderers."
            }
            Self::Selection => {
                "Click a column header, row number or cell. Arrow keys move the focus border without changing selection."
            }
            Self::Layout => {
                "Name stays 180px wide; Number and Bool share remaining width at 2:1. Drag a header's right edge to resize."
            }
            Self::Virtualization => {
                "2,000 rows × 40 columns. Scroll horizontally and vertically; row/column ranges and visible cell count follow the viewport."
            }
            Self::Disabled => {
                "Initially disabled. Enable to select and navigate, then disable again: selection stays while input and hover stop."
            }
        }
    }
}

impl Plugin for TableDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            app.register_widgetry_table::<DemoRow>();
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &String| {
                bsn_list![(Text({ value.clone() }))]
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &u32| {
                bsn_list![(Text({ format!("{value}") }))]
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &bool| {
                bsn_list![(Text({ if *value { "true" } else { "false" } }))]
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|value: &ProgressValue| {
                let progress_percent = (value.0 * 100.0).clamp(0.0, 100.0);
                bsn_list![(Node {width: percent(100), align_items: AlignItems::Center, column_gap: px(8)} Children [
                    (Node {width: px(90), height: px(10), overflow: Overflow::clip()} BackgroundColor(Color::srgb(0.24, 0.27, 0.32)) Children [
                        (Node {width: percent(progress_percent), height: percent(100)} BackgroundColor(Color::srgb(0.22, 0.62, 0.90))),
                    ]),
                    (Text({format!("{progress_percent:.0}%")})),
                ])]
            }))?;
            app.register_table_cell_renderer(WidgetryTableCellRenderer::new(|_: &StarValue| bsn_list![(
                @WidgetryIcon { @path: {GalleryIcon::ButtonStar.path()}, @max_size: {Some(UVec2::new(18, 18))} }
                Node {width: px(18), height: px(18)}
            )]))?;
            app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(
                |value: &String| bsn_list![(Text({ value.clone() }))],
            ))?;
            app.register_table_header_renderer(WidgetryTableHeaderRenderer::new(|value: &CustomHeader| bsn_list![(
                Node {align_items: AlignItems::Center, column_gap: px(8)} Children [
                    (@WidgetryIcon { @path: {GalleryIcon::ButtonStar.path()}, @max_size: {Some(UVec2::new(16, 16))} } Node {width: px(16), height: px(16)}),
                    (Text({value.0.clone()})),
                ]
            )]))?;
            let sources = sources(app.world_mut())?;
            app.insert_resource(sources)
                .add_observer(on_table_event)
                .add_observer(refresh_theme)
                .add_systems(PostUpdate, update_status.after(UiSystems::Layout));
            Ok(())
        })();
        if let Err(error) = result {
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(error) });
        }
    }
}
