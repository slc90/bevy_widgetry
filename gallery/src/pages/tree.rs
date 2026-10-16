use crate::gallery::{GalleryPage, mount_page, unmount_page};
use crate::i18n::{
    Language, LocalizedText, TextKey, localized, localized_args, localized_binding, tr, tr_args,
};
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::ui::{InteractionDisabled, UiSystems};
use bevy::ui_widgets::Activate;
use bevy::window::RequestRedraw;
use bevy_widgetry::button::WidgetryButton;
use bevy_widgetry::list_view::{WidgetryListViewItem, WidgetryListViewSystems};
use bevy_widgetry::tree::{
    WidgetryTreeAppExt, WidgetryTreeChildrenState, WidgetryTreeEvent, WidgetryTreeEventKind,
    WidgetryTreeModel, WidgetryTreeNode, WidgetryTreeRenderer, WidgetryTreeView,
};
use std::time::Duration;

pub(crate) struct TreeDemoPlugin;

pub(crate) fn color_examples(world: &World) -> impl Scene + use<> {
    let source = world.resource::<DemoSources>().models[0];
    let view = move || bsn! { @WidgetryTreeView { @source: source } Node { width: percent(100), height: px(160) } };
    crate::color_showcase::pair(
        TextKey::Tree,
        TextKey::OnlyItemSelectedBackground0F766EMovePointer2,
        view(),
        view(),
        |world, entity, apply| {
            use bevy_widgetry::tree::WidgetryTreeColorOverrides;
            if !apply {
                return WidgetryTreeColorOverrides::clear_in_world(world, entity);
            }
            let mut colors = WidgetryTreeColorOverrides::default();
            colors.item.selected.background = Some(crate::color_showcase::TEAL);
            WidgetryTreeColorOverrides::set_in_world(world, entity, colors)
        },
    )
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub(crate) struct DemoSources {
    models: [Entity; 5],
    roots: [Entity; 5],
    next_shared_file: usize,
}

#[derive(Component)]
struct BasicNode(TextKey);

#[derive(Component)]
struct Folder(TextKey);

#[derive(Component)]
struct File {
    label: String,
    bytes: usize,
}

#[derive(Component)]
struct SharedFile {
    index: usize,
    edits: usize,
}

#[derive(Component)]
struct TreeDemo;

#[derive(Component)]
struct DemoTreeView(usize);

#[derive(Component, Clone, Copy)]
struct TreeStatus {
    source: Entity,
    view_kind: usize,
}

#[derive(Component)]
struct LoadDeadline(Duration);

#[derive(Component, Clone, Copy)]
struct TreeAction {
    source: Entity,
    view_kind: usize,
    kind: Action,
}

#[derive(Clone, Copy, Debug)]
enum Action {
    ToggleDisabled,
    SelectLast,
    Edit,
    Insert,
    Remove,
    Reparent,
}

pub(crate) fn scene(sources: [Entity; 5]) -> impl Scene {
    bsn! {
        template(|_| Ok(TreeDemo))
        Node { width: percent(100), flex_shrink: 0.0, display: Display::Grid, grid_template_columns: vec![RepeatedGridTrack::flex(2, 1.0)], grid_template_rows: vec![RepeatedGridTrack::auto(3)], column_gap: px(24), row_gap: px(20) }
        Children [
            @panel(sources[4], 4,TextKey::TreeSharedA,TextKey::TreeSharedDescription)--
            @panel(sources[4], 5,TextKey::TreeSharedB,TextKey::TreeSharedDescription)--
            @panel(sources[0], 0,TextKey::BasicTree,TextKey::ClickArrowsToExpandClickContentTo)--
            @panel(sources[1], 1,TextKey::EcsHeterogeneousTree,TextKey::FolderAndFileComponentsDispatchDifferentRenderers)--
            @panel(sources[2], 2,TextKey::LazyLoadingTree,TextKey::ExpandRemoteFolderUnloadedLoadingLoadedCollapse)--
            @panel(sources[3], 3,TextKey::VirtualizedTree,TextKey::Expand10000FilesWheelPagedownScroll)
        ]
    }
}

fn panel(source: Entity, kind: usize, title: TextKey, description: TextKey) -> impl Scene {
    let controls = [
        (TextKey::EnableDisable, Action::ToggleDisabled),
        (TextKey::SelectLastApi, Action::SelectLast),
    ]
    .into_iter()
    .chain(
        (kind >= 4)
            .then_some([
                (TextKey::EditSelected, Action::Edit),
                (TextKey::TreeInsertFile, Action::Insert),
                (TextKey::RemoveSelected, Action::Remove),
                (TextKey::TreeReparentSelected, Action::Reparent),
            ])
            .into_iter()
            .flatten(),
    )
    .map(move |(label, action)| {
        bsn! {
            @WidgetryButton
            template(move |_| Ok(TreeAction { source, view_kind: kind, kind: action }))
            Name({format!("Tree{action:?}{kind}")})
            on(operate)
            Children [@localized(label) bevy_widgetry::text::WidgetryText]
        }
    })
    .collect::<Vec<_>>();
    bsn! {
        Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column, row_gap: px(8) }
        Children [
            @localized(title) bevy_widgetry::text::WidgetryText--
            @localized(description) bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(14.0) }--
            template(move |_| Ok(TreeStatus { source, view_kind: kind })) Name({format!("TreeStatus{kind}")}) Text("") bevy_widgetry::text::WidgetryText TextFont { font_size: FontSize::Px(14.0) }--
            @WidgetryTreeView { @source: source } template(move |_| Ok(DemoTreeView(kind))) Name({format!("TreeView{kind}")}) Node { width: percent(100), height: px(if kind == 5 { 194 } else { 258 }), flex_shrink: 0.0 }--
            Node { column_gap: px(8), row_gap: px(8), flex_wrap: FlexWrap::Wrap } Children [{controls}]
        ]
    }
}

fn operate(event: On<Activate>, actions: Query<&TreeAction>, mut commands: Commands) {
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    let action = *action;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world.get_entity(action.source).is_err() {
            return Ok(());
        }
        match action.kind {
            Action::ToggleDisabled => {
                let view = world
                    .query::<(Entity, &DemoTreeView)>()
                    .iter(world)
                    .find(|(_, view)| view.0 == action.view_kind)
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
            Action::Edit | Action::Insert | Action::Remove | Action::Reparent => {
                edit_shared(world, action)?;
            }
        }
        Ok(())
    });
}

fn edit_shared(world: &mut World, action: TreeAction) -> Result {
    let Some(model) = world.get::<WidgetryTreeModel>(action.source) else {
        error!(source = ?action.source, "Gallery 共享 Tree 缺失 Model");
        return Err(BevyError::error("Gallery shared Tree Model missing"));
    };
    let root = model.root();
    let selected = model.state().selected();
    let folder = world
        .get::<Children>(root)
        .and_then(|children| children.first())
        .copied();
    let Some(folder) = folder else {
        error!(?root, "Gallery 共享 Tree 缺失目录");
        return Err(BevyError::error("Gallery shared Tree folder missing"));
    };
    if matches!(action.kind, Action::Insert) {
        let index = world.resource::<DemoSources>().next_shared_file;
        world.resource_mut::<DemoSources>().next_shared_file += 1;
        world.spawn((
            WidgetryTreeNode,
            SharedFile { index, edits: 0 },
            Name::new(format!("Shared file {index:02}")),
            ChildOf(folder),
        ));
        WidgetryTreeModel::expand(world, action.source, folder)?;
        info!(source = ?action.source, index, "插入共享 Tree 文件");
        return Ok(());
    }
    let Some(node) = selected.filter(|node| world.get::<SharedFile>(*node).is_some()) else {
        return Ok(());
    };
    match action.kind {
        Action::Edit => {
            if let Some(mut file) = world.get_mut::<SharedFile>(node) {
                file.edits += 1;
            }
        }
        Action::Remove => {
            world.despawn(node);
        }
        Action::Reparent => {
            let parent = if world
                .get::<ChildOf>(node)
                .is_some_and(|parent| parent.parent() == folder)
            {
                root
            } else {
                folder
            };
            world.entity_mut(node).insert(ChildOf(parent));
        }
        _ => return Ok(()),
    }
    info!(source = ?action.source, ?node, action = ?action.kind, "修改共享 Tree 文件");
    Ok(())
}

fn on_tree_event(
    event: On<WidgetryTreeEvent>,
    sources: Option<Res<DemoSources>>,
    mut commands: Commands,
) {
    let Some(sources) = sources else {
        return;
    };
    if !sources.models.contains(&event.entity) {
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
            commands.queue(move |world: &mut World| -> Result {
                if world.get_entity(node).is_err() {
                    return Ok(());
                }
                if let Some(mut folder) = world.get_mut::<Folder>(node) {
                    folder.0 = TextKey::RemoteFolderLoading;
                } else {
                    error!(?node, "Gallery lazy Tree node 缺少 Folder");
                    return Err(BevyError::error("Gallery lazy Tree Folder missing"));
                }
                let deadline =
                    world.resource::<Time<Real>>().elapsed() + Duration::from_millis(800);
                world.entity_mut(node).insert(LoadDeadline(deadline));
                info!(?node, "开始加载 Tree children");
                Ok(())
            });
        }
    }
}

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
            folder.0 = TextKey::RemoteFolderLoaded;
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

fn owner(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryTreeView>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn update_status(world: &mut World) {
    let language = *world.resource::<Language>();
    let statuses = world
        .query::<(Entity, &TreeStatus)>()
        .iter(world)
        .map(|(entity, status)| (entity, status.source, status.view_kind))
        .collect::<Vec<_>>();
    let views = world
        .query::<(Entity, &DemoTreeView)>()
        .iter(world)
        .map(|(entity, view)| (view.0, entity))
        .collect::<Vec<_>>();
    let rows = world
        .query::<(Entity, &WidgetryListViewItem)>()
        .iter(world)
        .filter_map(|(entity, item)| owner(world, entity).map(|view| (view, item.index)))
        .collect::<Vec<_>>();
    for (entity, source, view_kind) in statuses {
        let Some((_, view)) = views.iter().find(|(current, _)| *current == view_kind) else {
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
            .unwrap_or_else(|| tr(language, TextKey::None).into());
        let indices = rows
            .iter()
            .filter(|(root, _)| root == view)
            .map(|(_, index)| *index)
            .collect::<Vec<_>>();
        let label = tr_args(
            language,
            TextKey::TreeStatus,
            &[
                ("visible", tree.visible_items().len().to_string()),
                ("rows", indices.len().to_string()),
                ("start", format!("{:?}", indices.iter().min())),
                ("end", format!("{:?}", indices.iter().max())),
                ("selected", selected),
                (
                    "state",
                    tr(
                        language,
                        if world.get::<InteractionDisabled>(*view).is_some() {
                            TextKey::Disabled
                        } else {
                            TextKey::Enabled
                        },
                    )
                    .into(),
                ),
            ],
        );
        if let Some(mut text) = world.get_mut::<Text>(entity)
            && text.0 != label
        {
            text.0 = label;
        }
    }
}

fn sources(world: &mut World) -> DemoSources {
    let mut roots = [Entity::PLACEHOLDER; 5];
    let models = std::array::from_fn(|kind| {
        let root = world.spawn_empty().id();
        roots[kind] = root;
        match kind {
            0 => {
                let workspace = world
                    .spawn((
                        WidgetryTreeNode,
                        BasicNode(TextKey::Workspace),
                        Name::new("Workspace"),
                        ChildOf(root),
                    ))
                    .id();
                let projects = world
                    .spawn((
                        WidgetryTreeNode,
                        BasicNode(TextKey::Projects),
                        Name::new("Projects"),
                        ChildOf(workspace),
                    ))
                    .id();
                world.spawn((
                    WidgetryTreeNode,
                    BasicNode(TextKey::Widgetry),
                    Name::new("Widgetry"),
                    ChildOf(projects),
                ));
                world.spawn((
                    WidgetryTreeNode,
                    BasicNode(TextKey::Photos),
                    Name::new("Photos"),
                    ChildOf(workspace),
                ));
                world.spawn((
                    WidgetryTreeNode,
                    BasicNode(TextKey::Notes),
                    Name::new("Notes"),
                    ChildOf(root),
                ));
            }
            1 => {
                let folder = world
                    .spawn((
                        WidgetryTreeNode,
                        Folder(TextKey::Assets),
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
                    Folder(TextKey::RemoteFolderUnloaded),
                    Name::new("Remote"),
                    WidgetryTreeChildrenState::Unknown,
                    ChildOf(root),
                ));
            }
            3 => {
                let folder = world
                    .spawn((
                        WidgetryTreeNode,
                        Folder(TextKey::Text10000Files),
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
            _ => {
                let folder = world
                    .spawn((
                        WidgetryTreeNode,
                        Folder(TextKey::TreeSharedFolder),
                        Name::new("Shared folder"),
                        ChildOf(root),
                    ))
                    .id();
                for index in 0..64 {
                    world.spawn((
                        WidgetryTreeNode,
                        SharedFile { index, edits: 0 },
                        Name::new(format!("Shared file {index:02}")),
                        ChildOf(folder),
                    ));
                }
            }
        }
        world.spawn(WidgetryTreeModel::new(root)).id()
    });
    DemoSources {
        models,
        roots,
        next_shared_file: 64,
    }
}

impl Plugin for TreeDemoPlugin {
    fn build(&self, app: &mut App) {
        let result = (|| -> Result<(), BevyError> {
            app.register_renderer::<BasicNode>(WidgetryTreeRenderer::new(
                |_, node: &BasicNode| bsn_list!{@localized(node.0) bevy_widgetry::text::WidgetryText},
            ))?;
            app.register_renderer::<Folder>(WidgetryTreeRenderer::new(|_, node: &Folder| {
                bsn_list!{@localized_binding(LocalizedText::new(TextKey::TreeFolder).with_key_arg("label", node.0)) bevy_widgetry::text::WidgetryText}
            }))?;
            app.register_renderer::<File>(WidgetryTreeRenderer::new(|_, node: &File| {
                bsn_list!{@localized_args(TextKey::TreeFile, vec![("label", node.label.clone()), ("bytes", node.bytes.to_string())]) bevy_widgetry::text::WidgetryText}
            }))?;
            app.register_renderer::<SharedFile>(WidgetryTreeRenderer::new(|_, node: &SharedFile| {
                bsn_list!{@localized_args(TextKey::TreeSharedFile, vec![("index", node.index.to_string()), ("edits", node.edits.to_string())]) bevy_widgetry::text::WidgetryText}
            }))?;
            app.register_type::<DemoSources>()
                .add_systems(OnEnter(GalleryPage::Tree), enter)
                .add_systems(OnExit(GalleryPage::Tree), exit)
                .add_observer(on_tree_event)
                .add_systems(Update, load_children.run_if(in_state(GalleryPage::Tree)))
                .add_systems(
                    PostUpdate,
                    update_status
                        .after(WidgetryListViewSystems::Reconcile)
                        .before(UiSystems::Prepare)
                        .run_if(in_state(GalleryPage::Tree)),
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
    let sources = sources(world);
    let models = sources.models;
    world.insert_resource(sources);
    if let Err(error) = mount_page(world, GalleryPage::Tree, bsn_list! { @scene(models) }) {
        release_sources(world);
        return Err(error);
    }
    Ok(())
}

fn release_sources(world: &mut World) {
    if let Some(sources) = world.get_resource::<DemoSources>() {
        let (models, roots) = (sources.models, sources.roots);
        for model in models {
            world.despawn(model);
        }
        for root in roots {
            world.despawn(root);
        }
        world.remove_resource::<DemoSources>();
    }
}

fn exit(world: &mut World) -> Result {
    unmount_page(world, GalleryPage::Tree)?;
    release_sources(world);
    Ok(())
}
