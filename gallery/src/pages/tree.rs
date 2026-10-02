use bevy::app::Propagate;
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::{InteractionDisabled, UiSystems};
use bevy::ui_widgets::Activate;
use bevy::window::RequestRedraw;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::list_view::{WidgetryListViewItem, WidgetryListViewSystems};
use bevy_widgetry::style::{ForegroundColor, ThemeChanged, ThemeMode};
use bevy_widgetry::tree::{
    WidgetryTreeAppExt, WidgetryTreeChildrenState, WidgetryTreeEvent, WidgetryTreeEventKind,
    WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeRenderer, WidgetryTreeView,
};
use std::time::Duration;

/// Gallery 自有业务 Component 与 lazy loader，不进入 Widgetry 库。
pub(crate) struct TreeDemoPlugin;

/// 四个演示拥有独立 ECS hierarchy 与 Tree model，页面切换保留 state。
#[derive(Resource)]
pub(crate) struct DemoSources(pub(crate) [Entity; 4]);

/// Basic Tree 的同构业务 label。
#[derive(Component)]
struct BasicNode(String);

/// Folder renderer 与 File renderer 使用不同 Component 匹配。
#[derive(Component)]
struct Folder(String);

/// File 的业务内容不存放在 Tree marker 中。
#[derive(Component)]
struct File {
    /// 业务文件名。
    label: String,
    /// 示例大小，供异构 renderer 显示。
    bytes: usize,
}

/// 页面自有 foreground propagation 边界。
#[derive(Component)]
struct TreeDemo;

/// status text 绑定独立 source，不从物理 row 推断 logical selection。
#[derive(Component, Clone, Copy)]
struct TreeStatus(Entity);

/// lazy 请求从发起时计算 deadline，不计入请求之前的 idle frame 时间。
#[derive(Component)]
struct LoadDeadline(Duration);

/// 示例 Button 的语义目标，不依赖当前页面显示状态。
#[derive(Component, Clone, Copy)]
struct TreeAction {
    /// 目标示例 source。
    source: Entity,
    /// Button 的离散演示操作。
    kind: Action,
}

/// 公开 API 操作与用户 pointer 输入分开展示。
#[derive(Clone, Copy)]
enum Action {
    /// 整体 disabled 不修改 model state。
    ToggleDisabled,
    /// 程序选择仍在 disabled 时可用。
    SelectLast,
}

/// 四个有界 viewport 展示基本、异构、lazy 和大量节点场景。
pub(crate) fn scene(sources: [Entity; 4]) -> impl Scene {
    bsn! {
        template(|_| Ok(TreeDemo))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node { width: percent(100), height: percent(100), display: Display::Grid, grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.0)], grid_template_rows: vec![RepeatedGridTrack::flex(2, 1.0)], column_gap: px(24), row_gap: px(20) }
        Children [
            panel(sources[0], 0, "Basic Tree", "Click arrows to expand; click content to select. Arrow/Home/End + Space/Enter navigate."),
            panel(sources[1], 1, "ECS Heterogeneous Tree", "Folder and File Components dispatch different renderers. Business data stays on ECS nodes."),
            panel(sources[2], 2, "Lazy Loading Tree", "Expand Remote folder: unloaded → loading → loaded. Collapse/re-expand keeps one request."),
            panel(sources[3], 3, "Virtualized Tree", "Expand 10,000 files; wheel/PageDown scroll. Only viewport rows exist."),
        ]
    }
}

/// 每个 panel 都有可观察 status，所有 Tree 视觉复用 ListView/Button。
fn panel(
    source: Entity,
    kind: usize,
    title: &'static str,
    description: &'static str,
) -> impl Scene {
    bsn! {
        Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            Text(title),
            (Text(description) TextFont { font_size: FontSize::Px(14.0) }),
            (template(move |_| Ok(TreeStatus(source))) template(move |_| Ok(Name::new(format!("TreeStatus{kind}")))) Text("") TextFont { font_size: FontSize::Px(14.0) }),
            (@WidgetryTreeView { @source: source } template(move |_| Ok(Name::new(format!("TreeView{kind}")))) Node { width: percent(100), height: px(258), flex_shrink: 0.0 }),
            (Node { column_gap: px(8) } Children [
                (@WidgetryButton template(move |_| Ok(TreeAction { source, kind: Action::ToggleDisabled })) template(move |_| Ok(Name::new(format!("TreeToggleDisabled{kind}")))) on(operate) Children [Text("Enable / Disable")]),
                (@WidgetryButton template(move |_| Ok(TreeAction { source, kind: Action::SelectLast })) on(operate) Children [Text("Select last (API)")]),
            ]),
        ]
    }
}

/// 在真实 Button Activate 后执行公开 model API，并在修改确认后记录结果。
fn operate(event: On<Activate>, actions: Query<&TreeAction>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        match action.kind {
            Action::ToggleDisabled => {
                let view = world
                    .query::<(Entity, &WidgetryTreeView)>()
                    .iter(world)
                    .find(|(_, view)| view.source() == action.source)
                    .map(|(entity, _)| entity);
                let Some(view) = view else {
                    error!(source = ?action.source, "Gallery Tree 示例缺少 TreeView");
                    return Err(BevyError::error("Gallery TreeView missing"));
                };
                let disabled = world.get::<InteractionDisabled>(view).is_none();
                if disabled {
                    world.entity_mut(view).insert(InteractionDisabled);
                } else {
                    world.entity_mut(view).remove::<InteractionDisabled>();
                }
                info!(?view, disabled, "设置 Tree 整体 disabled");
            }
            Action::SelectLast => {
                let node = world
                    .get::<WidgetryTreeModel>(action.source)
                    .and_then(|tree| tree.visible_items().last())
                    .map(|item| item.entity);
                if let Some(node) = node
                    && WidgetryTreeModel::select(world, action.source, Some(node))?
                {
                    info!(source = ?action.source, ?node, "程序化设置 Tree selection");
                }
            }
        }
        Ok(())
    });
}

/// Tree 的展开/收起/selection 变化日志来自语义 event；lazy 请求交给应用自有 loader。
fn on_tree_event(event: On<WidgetryTreeEvent>, sources: Res<DemoSources>, mut commands: Commands) {
    if !sources.0.contains(&event.entity) {
        return;
    }
    match event.kind {
        WidgetryTreeEventKind::Expanded(node) => {
            info!(source = ?event.entity, ?node, "展开 Tree node")
        }
        WidgetryTreeEventKind::Collapsed(node) => {
            info!(source = ?event.entity, ?node, "收起 Tree node")
        }
        WidgetryTreeEventKind::Selected(node) => {
            info!(source = ?event.entity, ?node, "选择 Tree node")
        }
        WidgetryTreeEventKind::ChildrenRequested(node) => {
            commands.queue(move |world: &mut World| {
                if let Some(mut folder) = world.get_mut::<Folder>(node) {
                    folder.0 = "Remote folder (loading...)".into();
                } else {
                    error!(?node, "Gallery lazy Tree node 缺少 Folder");
                    return;
                }
                let deadline =
                    world.resource::<Time<Real>>().elapsed() + Duration::from_millis(800);
                world.entity_mut(node).insert(LoadDeadline(deadline));
                info!(?node, "开始加载 Tree children");
            });
        }
    }
}

/// 在真实在途加载期间驱动后续 frame，不改变 Gallery 的 desktop_app 运行模式。
fn load_children(
    time: Res<Time<Real>>,
    pending: Query<(Entity, &LoadDeadline)>,
    mut redraw: MessageWriter<RequestRedraw>,
    mut commands: Commands,
) {
    for (node, deadline) in &pending {
        if time.elapsed() < deadline.0 {
            redraw.write(RequestRedraw);
            continue;
        }
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let Some(mut folder) = world.get_mut::<Folder>(node) else {
                return Ok(());
            };
            folder.0 = "Remote folder (loaded)".into();
            for index in 0..12 {
                world.spawn((
                    WidgetryTreeNode,
                    File {
                        label: format!("Remote file {index:02}.txt"),
                        bytes: (index + 1) * 128,
                    },
                    ChildOf(node),
                ));
            }
            world.entity_mut(node).remove::<LoadDeadline>();
            WidgetryTreeChildrenState::set_loaded(world, node)?;
            info!(?node, children = 12, "Tree children 加载完成");
            Ok(())
        });
    }
}

/// 根据真实 hierarchy 统计当前 TreeView 的 physical row，source 仍保有完整 logical state。
fn owner(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryTreeView>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

/// 在 UI 消费前更新公开 state 的展示，不新增 interaction 日志 polling。
fn update_status(world: &mut World) {
    let statuses = world
        .query::<(Entity, &TreeStatus)>()
        .iter(world)
        .map(|(entity, status)| (entity, status.0))
        .collect::<Vec<_>>();
    let views = world
        .query::<(Entity, &WidgetryTreeView)>()
        .iter(world)
        .map(|(entity, view)| (view.source(), entity))
        .collect::<Vec<_>>();
    let rows = world
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(world)
        .filter_map(|(entity, item)| owner(world, entity).map(|view| (view, item.index)))
        .collect::<Vec<_>>();
    for (entity, source) in statuses {
        let Some((_, view)) = views.iter().find(|(current, _)| *current == source) else {
            continue;
        };
        let Some(tree) = world.get::<WidgetryTreeModel>(source) else {
            continue;
        };
        let selected = tree
            .state()
            .selected()
            .map(|node| {
                world
                    .get::<Name>(node)
                    .map(|name| name.as_str().to_owned())
                    .unwrap_or_else(|| format!("{node:?}"))
            })
            .unwrap_or_else(|| "none".into());
        let indices = rows
            .iter()
            .filter(|(root, _)| root == view)
            .map(|(_, index)| *index)
            .collect::<Vec<_>>();
        let label = format!(
            "Visible: {} | rows: {} {:?}..{:?} | selected: {} | {}",
            tree.visible_items().len(),
            indices.len(),
            indices.iter().min(),
            indices.iter().max(),
            selected,
            if world.get::<InteractionDisabled>(*view).is_some() {
                "disabled"
            } else {
                "enabled"
            }
        );
        if let Some(mut text) = world.get_mut::<Text>(entity)
            && text.0 != label
        {
            text.0 = label;
        }
    }
}

/// 页面自有文字跟随 theme，业务内容和 expander 配色由 Widgetry 维护。
fn refresh_theme(
    event: On<ThemeChanged>,
    mut roots: Query<&mut Propagate<ForegroundColor>, With<TreeDemo>>,
) {
    for mut foreground in &mut roots {
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

/// 分配 ECS 业务 hierarchy，UI Scene 不复制这些 node。
fn sources(world: &mut World) -> [Entity; 4] {
    std::array::from_fn(|kind| {
        let root = world.spawn_empty().id();
        match kind {
            0 => {
                let workspace = world
                    .spawn((
                        WidgetryTreeNode,
                        BasicNode("Workspace".into()),
                        Name::new("Workspace"),
                        ChildOf(root),
                    ))
                    .id();
                let projects = world
                    .spawn((
                        WidgetryTreeNode,
                        BasicNode("Projects".into()),
                        Name::new("Projects"),
                        ChildOf(workspace),
                    ))
                    .id();
                world.spawn((
                    WidgetryTreeNode,
                    BasicNode("Widgetry".into()),
                    Name::new("Widgetry"),
                    ChildOf(projects),
                ));
                world.spawn((
                    WidgetryTreeNode,
                    BasicNode("Photos".into()),
                    Name::new("Photos"),
                    ChildOf(workspace),
                ));
                world.spawn((
                    WidgetryTreeNode,
                    BasicNode("Notes".into()),
                    Name::new("Notes"),
                    ChildOf(root),
                ));
            }
            1 => {
                let folder = world
                    .spawn((
                        WidgetryTreeNode,
                        Folder("Assets".into()),
                        Name::new("Assets"),
                        ChildOf(root),
                    ))
                    .id();
                for (label, bytes) in [("icon.svg", 512), ("theme.json", 1024)] {
                    world.spawn((
                        WidgetryTreeNode,
                        File {
                            label: label.into(),
                            bytes,
                        },
                        Name::new(label),
                        ChildOf(folder),
                    ));
                }
                world.spawn((
                    WidgetryTreeNode,
                    File {
                        label: "README.md".into(),
                        bytes: 2048,
                    },
                    Name::new("README.md"),
                    ChildOf(root),
                ));
            }
            2 => {
                world.spawn((
                    WidgetryTreeNode,
                    Folder("Remote folder (unloaded)".into()),
                    Name::new("Remote"),
                    WidgetryTreeChildrenState::Unknown,
                    ChildOf(root),
                ));
            }
            _ => {
                let folder = world
                    .spawn((
                        WidgetryTreeNode,
                        Folder("10,000 files".into()),
                        Name::new("Large folder"),
                        ChildOf(root),
                    ))
                    .id();
                for index in 0..10_000 {
                    world.spawn((
                        WidgetryTreeNode,
                        File {
                            label: format!("File {index:05}.txt"),
                            bytes: index,
                        },
                        ChildOf(folder),
                    ));
                }
            }
        }
        world.spawn(WidgetryTreeModel::new(root)).id()
    })
}

impl Plugin for TreeDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            app.register_renderer::<BasicNode>(WidgetryTreeRenderer::new(
                |_, node: &BasicNode| bsn_list![(Text({ node.0.clone() }))],
            ))?;
            app.register_renderer::<Folder>(WidgetryTreeRenderer::new(|_, node: &Folder| {
                bsn_list![(Text({ format!("[Folder] {}", node.0) }))]
            }))?;
            app.register_renderer::<File>(WidgetryTreeRenderer::new(|_, node: &File| {
                bsn_list![(Text({ format!("[File] {}  ({} bytes)", node.label, node.bytes) }))]
            }))?;
            let models = sources(app.world_mut());
            app.insert_resource(DemoSources(models))
                .add_observer(on_tree_event)
                .add_observer(refresh_theme)
                .add_systems(Update, load_children)
                .add_systems(
                    PostUpdate,
                    update_status
                        .after(WidgetryListViewSystems::Reconcile)
                        .before(UiSystems::Prepare),
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
