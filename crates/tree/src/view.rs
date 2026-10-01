use crate::{WidgetryTreeEvent, WidgetryTreeEventKind, WidgetryTreeModel, WidgetryTreeVisibleItem};
use bevy::asset::AssetPath;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_log::widgetry_error;

/// BSN Tree 外壳；source 必须持续持有 WidgetryTreeModel，不允许修改其自动派生的 ListModel。
/// root 提供有界 layout 与祖先 TabGroup；内部 ListView 负责 navigation、scroll、virtualization。
/// model 的 Entity selection 是 authority，内部 ListView state 和物理 row 均为 projection。
/// 共享 source 的 view 共享 selection/expanded；InteractionDisabled 只限制该 view 的用户输入。
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTreeViewProps)]
pub struct WidgetryTreeView {
    /// 独立 Tree model source，创建后固定。
    source: Entity,
}

/// 一次性 BSN 构造配置；Scene 展开后不保留 props 副本。
pub struct WidgetryTreeViewProps {
    /// 必填，生命周期内持有 TreeModel。
    pub source: Entity,
    /// 固定 ListView 行高，默认 32 logical px，必须为有限正数。
    pub item_height: f32,
    /// 每级 hierarchy 缩进，默认 20 logical px，必须为有限非负数。
    pub indent_width: f32,
    /// expander 的展开/收起 SVG，可由调用方替换。
    pub icons: WidgetryTreeIcons,
    /// 生成业务内容，不承担 row wrapper 或 expander 行为。
    pub renderer: WidgetryListViewRenderer<WidgetryTreeVisibleItem>,
}

/// Tree 自有的 icon 配置；其余 style 直接复用 ListView 与 Button。
#[derive(Clone)]
pub struct WidgetryTreeIcons {
    /// 收起 node 使用的 SVG。
    pub expand: AssetPath<'static>,
    /// 展开 node 使用的 SVG。
    pub collapse: AssetPath<'static>,
}

/// expander 将官方 Button Activate 解析为 model node identity。
#[derive(Component, Clone, Copy)]
struct TreeExpander {
    /// node 所属 model，避免跨 Tree 操作。
    source: Entity,
    /// 业务 Entity，不使用 visible index。
    node: Entity,
}

/// source 的公开前置条件由 Tree 检查；不能将错误配置作为正常 filter。
pub(crate) fn validate_sources(
    views: Query<(Entity, &WidgetryTreeView)>,
    models: Query<(), With<WidgetryTreeModel>>,
) {
    for (entity, view) in &views {
        if !models.contains(view.source) {
            widgetry_error!(?entity, source = ?view.source, "TreeView source 缺失或没有 TreeModel");
            panic!("WidgetryTreeView requires a live TreeModel source");
        }
    }
}

/// 将 Entity selection 映射到 ListModel id，不将 hidden selection 清回 model。
pub(crate) fn project_selection(world: &mut World) {
    let views = world
        .query::<(Entity, &WidgetryTreeView)>()
        .iter(world)
        .map(|(root, view)| (root, view.source))
        .collect::<Vec<_>>();
    for (root, source) in views {
        let tree = invariant(world.get::<WidgetryTreeModel>(source), root);
        let model = invariant(
            world.get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source),
            root,
        );
        let selected = tree
            .state()
            .selected()
            .and_then(|node| tree.visible_index(node))
            .and_then(|index| model.id(index));
        let list = internal_list(world, root);
        let mut state = invariant(world.get_mut::<WidgetryListViewState>(list), root);
        if state.selected != selected {
            state.selected = selected;
            if selected.is_some() {
                state.active = selected;
            }
        }
    }
}

/// 用户 ListView 通知只在 source model 的 Entity selection 确实改变时转发。
pub(crate) fn on_selection(
    event: On<ValueChange<WidgetryListItemId>>,
    lists: Query<&ChildOf, With<WidgetryListView<WidgetryTreeVisibleItem>>>,
    roots: Query<&WidgetryTreeView>,
    mut commands: Commands,
) {
    let Ok(parent) = lists.get(event.source) else {
        return;
    };
    let root = parent.parent();
    let Ok(view) = roots.get(root) else {
        return;
    };
    let source = view.source;
    let id = event.value;
    commands.queue(move |world: &mut World| {
        if world.get::<InteractionDisabled>(root).is_some() {
            return;
        }
        let node = world
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .and_then(|model| model.get_by_id(id))
            .map(|item| item.entity);
        if let Some(node) = node
            && WidgetryTreeModel::select(world, source, Some(node))
        {
            world.trigger(WidgetryTreeEvent {
                entity: source,
                kind: WidgetryTreeEventKind::Selected(node),
            });
        }
    });
}

/// 官方 Button 处理 pressed/Activate；Tree 只处理展开语义并遵循 root disabled。
fn on_expand(event: On<Activate>, expanders: Query<&TreeExpander>, mut commands: Commands) {
    let Ok(expander) = expanders.get(event.entity) else {
        return;
    };
    let expander = *expander;
    let button = event.entity;
    commands.queue(move |world: &mut World| {
        let root = invariant(tree_root(world, button), button);
        if world.get::<InteractionDisabled>(root).is_some() {
            return;
        }
        WidgetryTreeModel::toggle_expand(world, expander.source, expander.node);
    });
}

/// 从 TreeView 固定 shell 定位唯一内部 ListView。
fn internal_list(world: &World, root: Entity) -> Entity {
    invariant(
        world.get::<Children>(root).and_then(|children| {
            children.iter().find(|&child| {
                world
                    .get::<WidgetryListView<WidgetryTreeVisibleItem>>(child)
                    .is_some()
            })
        }),
        root,
    )
}

/// 已识别 TreeView 缺失内部结构不能当作正常 observer filter。
fn invariant<T>(value: Option<T>, root: Entity) -> T {
    value.unwrap_or_else(|| {
        widgetry_error!(?root, "TreeView 内部 hierarchy 或 source invariant 失效");
        panic!("WidgetryTreeView invariant failed");
    })
}

/// 从当前 hierarchy 定位 TreeView，不以共享 source 推断 view ownership。
fn tree_root(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryTreeView>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

/// root disabled 镜像到内部交互边界，不修改 model 或业务 node。
pub(crate) fn sync_disabled(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryTreeView>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let disabled = world.get::<InteractionDisabled>(root).is_some();
        mirror_disabled(world, internal_list(world, root), disabled);
    }
    let buttons = world
        .query_filtered::<Entity, With<TreeExpander>>()
        .iter(world)
        .collect::<Vec<_>>();
    for button in buttons {
        let root = invariant(tree_root(world, button), button);
        mirror_disabled(
            world,
            button,
            world.get::<InteractionDisabled>(root).is_some(),
        );
    }
}

/// 只在边界实际改变时写 Component；disable 同时结束遗留 pressed state。
fn mirror_disabled(world: &mut World, entity: Entity, disabled: bool) {
    let current = world.get::<InteractionDisabled>(entity).is_some();
    if current != disabled {
        if disabled {
            world.entity_mut(entity).insert(InteractionDisabled);
        } else {
            world.entity_mut(entity).remove::<InteractionDisabled>();
        }
    }
    if disabled && world.get::<Pressed>(entity).is_some() {
        world.entity_mut(entity).remove::<Pressed>();
    }
}

/// disabled lifecycle 同帧 mirror，防止下一次 update 前 queued input 修改 selection。
pub(crate) fn on_disabled(
    event: On<Add, InteractionDisabled>,
    roots: Query<(), With<WidgetryTreeView>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        commands.queue(sync_disabled);
    }
}

/// remove observer 在 command 执行后按真实 state mirror，兼容 remove→insert。
pub(crate) fn on_enabled(
    event: On<Remove, InteractionDisabled>,
    roots: Query<(), With<WidgetryTreeView>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        commands.queue(sync_disabled);
    }
}

impl Default for WidgetryTreeIcons {
    fn default() -> Self {
        Self {
            expand: BuiltinIcon::TreeExpand.path(),
            collapse: BuiltinIcon::TreeCollapse.path(),
        }
    }
}

impl Default for WidgetryTreeViewProps {
    fn default() -> Self {
        Self {
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            indent_width: 20.0,
            icons: WidgetryTreeIcons::default(),
            renderer: WidgetryListViewRenderer::default(),
        }
    }
}

impl WidgetryTreeView {
    /// 读取创建后固定的 model source。
    pub fn source(&self) -> Entity {
        self.source
    }

    /// 配置检查后组合 ListView，业务内容与 Button 都由真实 row lifecycle 管理。
    fn scene(props: WidgetryTreeViewProps) -> impl Scene {
        if props.source == Entity::PLACEHOLDER
            || !props.indent_width.is_finite()
            || props.indent_width < 0.0
        {
            widgetry_error!(source = ?props.source, indent_width = props.indent_width, "TreeView 必须提供 source 与有限非负 indent_width");
            panic!("Invalid WidgetryTreeView configuration");
        }
        let source = props.source;
        let indent = props.indent_width;
        let icons = props.icons;
        let content = props.renderer;
        bsn! {
            WidgetryTreeView { source }
            Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column }
            Children [(
                @WidgetryListView::<WidgetryTreeVisibleItem> {
                    @source: source,
                    @item_height: {props.item_height},
                    @renderer: {WidgetryListViewRenderer::new(move |index, item: &WidgetryTreeVisibleItem| {
                        let node = item.entity;
                        let padding = f32::from(item.depth) * indent;
                        if !padding.is_finite() {
                            widgetry_error!(?node, "TreeView depth 与 indent_width 乘积超出有限范围");
                            panic!("TreeView indentation overflow");
                        }
                        let icon = if item.expanded { icons.collapse.clone() } else { icons.expand.clone() };
                        let visibility = if item.has_children { Visibility::Inherited } else { Visibility::Hidden };
                        let children = content.render(index, item);
                        bsn_list![(
                            Node { width: percent(100), align_items: AlignItems::Center, padding: UiRect::left(px(padding)), column_gap: px(6) }
                            Children [(
                                @WidgetryButton
                                template(move |_| Ok(TreeExpander { source, node }))
                                template(move |_| Ok(visibility))
                                Node { width: px(20), height: px(20), min_height: px(20), padding: UiRect::ZERO, flex_shrink: 0.0, align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                                on(on_expand)
                                Children [(
                                    @WidgetryIcon { @path: icon, @max_size: {Some(UVec2::splat(12))} }
                                    template(|_| Ok(Pickable::IGNORE))
                                    Node { width: px(12), height: px(12) }
                                )]
                            ), (Node { min_width: px(0), flex_grow: 1.0 } Children [{children}])]
                        )]
                    })},
                }
                Node { width: percent(100), height: percent(100), min_height: px(0) }
            )]
        }
    }
}
