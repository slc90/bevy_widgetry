use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, ScrollPosition};
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewAppExt,
    WidgetryListViewItem, WidgetryListViewPlugin, WidgetryListViewRenderer, WidgetryListViewState,
};
use bevy_widgetry::scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry::style::{ForegroundColor, ThemeChanged, ThemeMode};

/// 注册 Gallery 业务 type 并维护公开 state 的可观察文字。
pub(crate) struct ListViewDemoPlugin;

/// renderer 的业务内容；修改次数用于识别 revision 更新。
struct DemoItem {
    /// 不依赖当前 model index 的业务名称。
    label: String,
    /// 每次 Gallery update 操作递增。
    edits: usize,
}

/// 四类示例拥有独立 model，页面显隐不会重置 identity。
#[derive(Resource)]
pub(crate) struct DemoSources(pub(crate) [Entity; 4]);

/// 页面自有文字的 theme 传播边界。
#[derive(Component)]
struct ListViewDemo;

/// 将 root 或 status 绑定到同一示例。
#[derive(Component, Clone, Copy, PartialEq, Eq)]
struct DemoKind(usize);

/// 展示 logical state、public row range 和 selection 变化通知次数。
#[derive(Component, Default)]
struct DemoStatus {
    /// UI 与程序 API 实际 selection 变化的次数，不统计自动 repair。
    changes: usize,
}

/// Button 通过所属 panel 定位目标 ListView。
#[derive(Component, Clone, Copy)]
struct DemoAction(Action);

/// Gallery 操作仅调用已有公开 API。
#[derive(Clone, Copy, Debug)]
enum Action {
    Select,
    ClearSelection,
    VisibleUpdate,
    OffscreenUpdate,
    Insert,
    Remove,
    Move,
    Reset,
    Shrink,
    Scroll,
    ToggleDisabled,
    Resize,
}

/// 四类场景同屏，ListView 使用有界高度，按钮与说明在 viewport 外。
pub(crate) fn scene(sources: [Entity; 4]) -> impl Scene {
    bsn! {
        template(|_| Ok(ListViewDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node {
            width: percent(100), height: percent(100), display: Display::Grid,
            grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.0)],
            grid_template_rows: vec![RepeatedGridTrack::flex(2, 1.0)],
            column_gap: px(24), row_gap: px(16),
        }
        Children [
            section(sources[0], 0, "Small List", "Arrow/Home/End: active; Space/Enter: select. Click text or row."),
            section(sources[1], 1, "Virtualized Large List", "10,000 items; wheel/PageUp/PageDown; jump, edit, shrink and resize."),
            section(sources[2], 2, "Disabled ListView", "User input blocked; model, selection and scroll buttons remain available."),
            section(sources[3], 3, "Disabled ListViewItem", "Initially items 1/3 disabled. Arrow/click can activate; Space/Enter cannot select."),
        ]
    }
}

/// 每个 panel 同时持有 status 与操作，但 model 留在独立 source entity。
fn section(
    source: Entity,
    kind: usize,
    title: &'static str,
    description: &'static str,
) -> impl Scene {
    let controls: Vec<_> = [
        ("Set selected", Action::Select),
        ("Clear selection", Action::ClearSelection),
        ("Edit visible", Action::VisibleUpdate),
        ("Insert first", Action::Insert),
        ("Remove first", Action::Remove),
        ("Move first → last", Action::Move),
        ("Clear + repopulate", Action::Reset),
    ]
    .into_iter()
    .chain(match kind {
        1 => vec![
            ("Edit #7500", Action::OffscreenUpdate),
            ("Shrink to 3", Action::Shrink),
            ("Resize viewport", Action::Resize),
        ],
        2 => vec![("Scroll +96px", Action::Scroll)],
        3 => vec![
            ("Enable/disable #1", Action::ToggleDisabled),
            ("Scroll +96px", Action::Scroll),
        ],
        _ => vec![],
    })
    .map(|(label, action)| action_button(label, action))
    .collect();
    bsn! {
        template(move |_| Ok(DemoKind(kind)))
        Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text(title),
            (Text(description) TextFont { font_size: bevy::text::FontSize::Px(14.0) }),
            (
                template(move |_| Ok(DemoKind(kind)))
                template(|_| Ok(DemoStatus::default()))
                Text("") TextFont { font_size: bevy::text::FontSize::Px(14.0) }
            ),
            (
                @WidgetryListView::<DemoItem> {
                    @source: source,
                    @item_height: 32.0,
                    @renderer: {WidgetryListViewRenderer::new(|_, value: &DemoItem| {
                        let label = format!("{}  (edits: {})", value.label, value.edits);
                        bsn_list![(Node { width: percent(100) } Children [(Text({label}))])]
                    })},
                }
                template(move |_| Ok(DemoKind(kind)))
                template(move |_| Ok(Name::new(format!("ListViewDemo{kind}"))))
                Node { width: percent(100), height: px(162), flex_shrink: 0.0 }
                on(record_change)
            ),
            (Node { flex_direction: FlexDirection::Row, flex_wrap: FlexWrap::Wrap, column_gap: px(8), row_gap: px(8) } Children [{controls}]),
        ]
    }
}

/// 所有操作都由真实 Widgetry Button 的 Activate 驱动。
fn action_button(label: &'static str, action: Action) -> impl Scene {
    bsn! {
        @WidgetryButton
        template(move |_| Ok(DemoAction(action)))
        Node { align_items: AlignItems::Center, justify_content: JustifyContent::Center }
        on(operate)
        Children [(Text(label) TextFont { font_size: bevy::text::FontSize::Px(14.0) })]
    }
}

/// 向现有 model 追加新 entry；reset 保留 source 及其单调 id counter。
fn populate(model: &mut WidgetryListModel<DemoItem>, kind: usize) -> Result<(), BevyError> {
    let len = match kind {
        0 => 4,
        1 => 10_000,
        _ => 20,
    };
    for index in 0..len {
        model.push(DemoItem {
            label: format!("Item {index}"),
            edits: 0,
        })?;
    }
    if kind == 3 {
        model.set_disabled(1, true);
        model.set_disabled(3, true);
    }
    Ok(())
}

/// 通过 public hierarchy 判断 row/viewport 所属 root，不读取 private runtime。
fn belongs(entity: Entity, root: Entity, parents: &Query<&ChildOf>) -> bool {
    entity == root
        || parents
            .iter_ancestors(entity)
            .any(|ancestor| ancestor == root)
}

/// 将按钮映射到公开 model CRUD、selection API 与原生 ScrollPosition。
fn operate(
    event: On<Activate>,
    actions: Query<&DemoAction>,
    kinds: Query<&DemoKind>,
    parents: Query<&ChildOf>,
    mut lists: Query<(Entity, &DemoKind, &WidgetryListView<DemoItem>, &mut Node)>,
    mut models: Query<&mut WidgetryListModel<DemoItem>>,
    rows: Query<(Entity, &WidgetryListViewItem)>,
    mut viewports: Query<(Entity, &mut ScrollPosition), With<WidgetryScrollAreaViewport>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let Ok(action) = actions.get(event.entity) else {
        return Ok(());
    };
    let Some(kind) = parents
        .iter_ancestors(event.entity)
        .find_map(|ancestor| kinds.get(ancestor).ok())
    else {
        return Ok(());
    };
    let Some((root, _, view, mut node)) = lists
        .iter_mut()
        .find(|(_, candidate, _, _)| *candidate == kind)
    else {
        return Ok(());
    };
    let Ok(mut model) = models.get_mut(view.source()) else {
        return Ok(());
    };
    info!(entity = ?root, action = ?action.0, "执行 ListView programmatic 演示操作" );
    match action.0 {
        Action::ClearSelection => {
            WidgetryListView::<DemoItem>::clear_selection(&mut commands, root)
        }
        Action::Select => WidgetryListView::<DemoItem>::set_selected(
            &mut commands,
            root,
            match kind.0 {
                1 => 7500,
                2 => 10,
                _ => 1,
            },
        ),
        Action::VisibleUpdate | Action::OffscreenUpdate => {
            let index = if matches!(action.0, Action::OffscreenUpdate) {
                Some(7500)
            } else {
                rows.iter()
                    .filter(|(entity, _)| belongs(*entity, root, &parents))
                    .map(|(_, item)| item.index)
                    .min()
            };
            if let Some(value) = index
                .map(|index| model.get_mut(index))
                .transpose()?
                .flatten()
            {
                value.edits += 1;
            }
        }
        Action::Insert => {
            model.insert(
                0,
                DemoItem {
                    label: "Inserted item".into(),
                    edits: 0,
                },
            )?;
        }
        Action::Remove => {
            model.remove(0);
        }
        Action::Move => {
            if !model.is_empty() {
                let last = model.len() - 1;
                model.move_item(0, last);
            }
        }
        Action::Reset => {
            model.clear();
            populate(&mut model, kind.0)?;
        }
        Action::Shrink => {
            while model.len() > 3 {
                let last = model.len() - 1;
                model.remove(last);
            }
        }
        Action::Scroll => {
            for (entity, mut position) in &mut viewports {
                if belongs(entity, root, &parents) {
                    position.0.y += 96.0;
                }
            }
        }
        Action::ToggleDisabled => {
            if let Some(disabled) = model.is_disabled(1) {
                model.set_disabled(1, !disabled);
            }
        }
        Action::Resize => {
            node.height = if node.height == px(162) {
                px(258)
            } else {
                px(162)
            };
        }
    }
    Ok(())
}

/// 统计 UI 与程序 API 的实际 selection 变化；初始化与自动 repair 不通知。
fn record_change(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
    lists: Query<&DemoKind, With<WidgetryListView<DemoItem>>>,
    mut statuses: Query<(&DemoKind, &mut DemoStatus)>,
) {
    info!(entity = ?event.source, selected = ?event.value, "ListView selection 已改变" );
    let Ok(kind) = lists.get(event.source) else {
        return;
    };
    for (candidate, mut status) in &mut statuses {
        if candidate == kind {
            status.changes += 1;
        }
    }
}

/// 每帧从 public identity 与 hierarchy 计算数量和 range，文字本身也是 BRP 可读 state。
fn update_status(
    lists: Query<(
        Entity,
        &DemoKind,
        &WidgetryListView<DemoItem>,
        &WidgetryListViewState,
    )>,
    models: Query<&WidgetryListModel<DemoItem>>,
    rows: Query<(Entity, &WidgetryListViewItem)>,
    parents: Query<&ChildOf>,
    mut statuses: Query<(&DemoKind, &DemoStatus, &mut Text)>,
) {
    for (root, kind, view, state) in &lists {
        let Ok(model) = models.get(view.source()) else {
            continue;
        };
        let mut indices: Vec<_> = rows
            .iter()
            .filter(|(entity, _)| belongs(*entity, root, &parents))
            .map(|(_, item)| item.index)
            .collect();
        indices.sort_unstable();
        let start = indices.first().copied().unwrap_or(0);
        let end = indices.last().map_or(0, |index| index + 1);
        for (candidate, status, mut text) in &mut statuses {
            if candidate != kind {
                continue;
            }
            let value = format!(
                "Model items: {} | Rendered rows: {} | Rendered range: {start}..{end}\nselected: {:?} / {:?} | active: {:?} / {:?} | ValueChange: {}",
                model.len(),
                indices.len(),
                state.selected,
                state.selected.and_then(|id| model.index_of(id)),
                state.active,
                state.active.and_then(|id| model.index_of(id)),
                status.changes
            );
            if text.0 != value {
                text.0 = value;
            }
        }
    }
}

/// disabled root 保留 logical state，入口仅限制用户输入。
fn initialize_disabled(
    mut commands: Commands,
    lists: Query<(Entity, &DemoKind), Added<WidgetryListView<DemoItem>>>,
) {
    for (entity, kind) in &lists {
        if kind.0 == 2 {
            commands.entity(entity).insert(InteractionDisabled);
            WidgetryListView::<DemoItem>::set_selected(&mut commands, entity, 1);
        }
    }
}

/// 页面标题、说明和 status 跟随全局 theme，row foreground 由 library 维护。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<ListViewDemo>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

impl Plugin for ListViewDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            if !app.is_plugin_added::<WidgetryListViewPlugin>() {
                app.add_plugins(WidgetryListViewPlugin);
            }
            app.register_widgetry_list_view::<DemoItem>()?;
            let mut sources = [Entity::PLACEHOLDER; 4];
            for (kind, source) in sources.iter_mut().enumerate() {
                let mut model = WidgetryListModel::default();
                populate(&mut model, kind)?;
                *source = app.world_mut().spawn(model).id();
            }
            app.insert_resource(DemoSources(sources))
                .add_observer(refresh_theme)
                .add_systems(Update, initialize_disabled)
                .add_systems(PostUpdate, update_status.after(bevy::ui::UiSystems::Layout));

            Ok(())
        })();
        if let Err(error) = result {
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(error) });
        }
    }
}
