use crate::renderer::TreeContent;
use crate::{WidgetryTreeEvent, WidgetryTreeEventKind, WidgetryTreeModel, WidgetryTreeVisibleItem};
use bevy::asset::AssetPath;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

/// BSN Tree 外壳；source 必须持续持有 WidgetryTreeModel，不允许修改其自动派生的 ListModel。
/// root 提供有界 layout 与祖先 TabGroup；内部 ListView 负责 navigation、scroll、virtualization。
/// model 的 Entity selection 是 authority，内部 ListView state 和物理 row 均为 projection。
/// 共享 source 的 view 共享 selection/expanded；InteractionDisabled 只限制该 view 的用户输入。
/// 应用通过 WidgetryTreeAppExt 注册业务 Component renderer；每个 rendered node 必须恰好匹配一种。
#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTreeViewProps)]
#[require(ViewDiagnostics)]
pub struct WidgetryTreeView {
    /// 独立 Tree model source，创建后固定。
    source: Entity,
}

/// TreeView 负责 source、selection 和 disabled 的独立异常边界。
#[derive(Component, Default)]
struct ViewDiagnostics {
    source: FailureState,
    selection: FailureState,
    disabled: FailureState,
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
}

/// Tree 自有的 icon 配置；其余 style 直接复用 ListView 与 Button。
#[derive(Clone)]
pub struct WidgetryTreeIcons {
    /// 收起 node 使用的 SVG。
    pub expand: AssetPath<'static>,
    /// 展开 node 使用的 SVG。
    pub collapse: AssetPath<'static>,
}

/// fallible template 的私有输出，保留公开 Widget 与 Node 的 BSN patch 合并语义。
#[derive(Component)]
struct ValidatedConfig;

/// expander 将官方 Button Activate 解析为 model node identity。
#[derive(Component, Clone, Copy)]
#[require(ExpanderDiagnostics)]
struct TreeExpander {
    /// node 所属 model，避免跨 Tree 操作。
    source: Entity,
    /// 业务 Entity，不使用 visible index。
    node: Entity,
}

/// 被外部 reparent 的 expander 自己记录 ownership 异常，避免每帧刷日志。
#[derive(Component, Default)]
struct ExpanderDiagnostics(FailureState);

/// source 的公开前置条件由 Tree 检查；不能将错误配置作为正常 filter。
pub(crate) fn validate_sources(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, With<WidgetryTreeView>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if let Err(error) = validate_source(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }

    failure.map_or(Ok(()), Err)
}

/// 任何消费 source 的阶段都检查同一个 contract，不重复记录。
fn validate_source(world: &mut World, root: Entity) -> Result<(), BevyError> {
    let source = world
        .get::<WidgetryTreeView>(root)
        .map(|view| view.source)
        .unwrap_or(Entity::PLACEHOLDER);
    let result = if world.get::<WidgetryTreeModel>(source).is_some() {
        Ok(())
    } else {
        Err(BevyError::error(
            "WidgetryTreeView requires a live TreeModel source",
        ))
    };
    let Some(mut diagnostics) = world.get_mut::<ViewDiagnostics>(root) else {
        return result;
    };
    diagnostics.source.observe(result,
        |error| widgetry_error!(entity = ?root, ?source, %error, "TreeView source 缺失或没有 TreeModel"),
        || widgetry_info!(entity = ?root, ?source, "TreeView source 恢复正常"))
}

/// 将 Entity selection 映射到 ListModel id，不将 hidden selection 清回 model。
pub(crate) fn project_selection(world: &mut World) -> Result<(), BevyError> {
    let views = world
        .query::<(Entity, &WidgetryTreeView)>()
        .iter(world)
        .map(|(root, view)| (root, view.source))
        .collect::<Vec<_>>();

    let mut failure = None;
    for (root, source) in views {
        if let Err(error) = validate_source(world, root) {
            if failure.is_none() {
                failure = Some(error);
            }
            continue;
        }
        let result = (|| -> Result<(), BevyError> {
            let tree = invariant(world.get::<WidgetryTreeModel>(source), root)?;
            let model = invariant(
                world.get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source),
                root,
            )?;
            let selected = tree
                .state()
                .selected()
                .and_then(|node| tree.visible_index(node))
                .and_then(|index| model.id(index));
            let list = internal_list(world, root)?;
            let mut state = invariant(world.get_mut::<WidgetryListViewState>(list), root)?;
            if state.selected != selected {
                state.selected = selected;
                if selected.is_some() {
                    state.active = selected;
                }
            }

            Ok(())
        })();
        let result = if let Some(mut diagnostics) = world.get_mut::<ViewDiagnostics>(root) {
            diagnostics.selection.observe(
                result,
                |error| widgetry_error!(?root, %error, "TreeView selection projection 失败"),
                || widgetry_info!(?root, "TreeView selection projection 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
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
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world.get::<InteractionDisabled>(root).is_some() {
            return Ok(());
        }
        let node = world
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .and_then(|model| model.get_by_id(id))
            .map(|item| item.entity);
        if let Some(node) = node
            && WidgetryTreeModel::select(world, source, Some(node))?
        {
            world.trigger(WidgetryTreeEvent {
                entity: source,
                kind: WidgetryTreeEventKind::Selected(node),
            });
        }
        Ok(())
    });
}

/// 官方 Button 处理 pressed/Activate；Tree 只处理展开语义并遵循 root disabled。
fn on_expand(event: On<Activate>, expanders: Query<&TreeExpander>, mut commands: Commands) {
    let Ok(expander) = expanders.get(event.entity) else {
        return;
    };
    let expander = *expander;
    let button = event.entity;
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let root = tree_root(world, button).ok_or_else(|| {
            widgetry_error!(?button, "Tree expander 缺少所属 TreeView");
            BevyError::error("Tree expander has no owning TreeView")
        })?;
        if world.get::<InteractionDisabled>(root).is_some() {
            return Ok(());
        }
        WidgetryTreeModel::toggle_expand(world, expander.source, expander.node)?;
        Ok(())
    });
}

/// 从 TreeView 固定 shell 定位唯一内部 ListView。
fn internal_list(world: &World, root: Entity) -> Result<Entity, BevyError> {
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
fn invariant<T>(value: Option<T>, _root: Entity) -> Result<T, BevyError> {
    value.ok_or_else(|| BevyError::error("WidgetryTreeView invariant failed"))
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
pub(crate) fn sync_disabled(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, With<WidgetryTreeView>>()
        .iter(world)
        .collect::<Vec<_>>();
    let buttons = world
        .query_filtered::<Entity, With<TreeExpander>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        let result = (|| -> Result<(), BevyError> {
            let disabled = world.get::<InteractionDisabled>(root).is_some();
            mirror_disabled(world, internal_list(world, root)?, disabled);
            for &button in &buttons {
                if tree_root(world, button) == Some(root) {
                    mirror_disabled(world, button, disabled);
                }
            }
            Ok(())
        })();
        let result = if let Some(mut diagnostics) = world.get_mut::<ViewDiagnostics>(root) {
            diagnostics.disabled.observe(
                result,
                |error| widgetry_error!(?root, %error, "TreeView disabled hierarchy 失效"),
                || widgetry_info!(?root, "TreeView disabled hierarchy 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    for button in buttons {
        let result = if tree_root(world, button).is_some() {
            Ok(())
        } else {
            Err(BevyError::error("Tree expander has no owning TreeView"))
        };
        let result = if let Some(mut diagnostics) = world.get_mut::<ExpanderDiagnostics>(button) {
            diagnostics.0.observe(
                result,
                |error| widgetry_error!(?button, %error, "Tree expander ownership 失效"),
                || widgetry_info!(?button, "Tree expander ownership 恢复正常"),
            )
        } else {
            result
        };
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
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

/// 完整 shell 的 disabled lifecycle 同帧 mirror；BSN 初始 root 由首次 PreUpdate 同步。
/// Scene 会先应用 root Component，再应用已分配的 children Component；不能在中间态执行严格同步。
pub(crate) fn on_disabled(
    event: On<Add, InteractionDisabled>,
    roots: Query<&Children, With<WidgetryTreeView>>,
    lists: Query<(), With<WidgetryListView<WidgetryTreeVisibleItem>>>,
    mut commands: Commands,
) {
    if roots
        .get(event.entity)
        .is_ok_and(|children| children.iter().any(|child| lists.contains(child)))
    {
        commands.queue(sync_disabled);
    }
}

/// 完整 shell 的 remove observer 在 command 执行后按真实 state mirror，兼容 remove→insert。
pub(crate) fn on_enabled(
    event: On<Remove, InteractionDisabled>,
    roots: Query<&Children, With<WidgetryTreeView>>,
    lists: Query<(), With<WidgetryListView<WidgetryTreeVisibleItem>>>,
    mut commands: Commands,
) {
    if roots
        .get(event.entity)
        .is_ok_and(|children| children.iter().any(|child| lists.contains(child)))
    {
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
        let source = props.source;
        let indent = props.indent_width;
        let icons = props.icons;
        bsn! {
            WidgetryTreeView { source }
            template(move |_| {
                if source == Entity::PLACEHOLDER || !indent.is_finite() || indent < 0.0 {
                    widgetry_error!(source = ?source, indent_width = indent, "TreeView 必须提供 source 与有限非负 indent_width");
                    return Err(bevy_widgetry_core::scene::logged_error("Invalid WidgetryTreeView configuration"));
                }
                Ok(ValidatedConfig)
            })
            Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column }
            Children [(
                @WidgetryListView::<WidgetryTreeVisibleItem> {
                    @source: source,
                    @item_height: {props.item_height},
                    @renderer: {WidgetryListViewRenderer::new(move |_, item: &WidgetryTreeVisibleItem| {
                        let node = item.entity;
                        let padding = f32::from(item.depth) * indent;
                        let icon = if item.expanded { icons.collapse.clone() } else { icons.expand.clone() };
                        let visibility = if item.has_children { Visibility::Inherited } else { Visibility::Hidden };
                        bsn_list![(
                            template(move |_| {
                                if !padding.is_finite() {
                                    return Err(BevyError::error("TreeView indentation overflow"));
                                }
                                Ok(Node { width: percent(100), align_items: AlignItems::Center, padding: UiRect::left(px(padding)), column_gap: px(6), ..default() })
                            })
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
                            ), (template(move |_| Ok(TreeContent { node })) Node { min_width: px(0), flex_grow: 1.0 })]
                        )]
                    })},
                }
                Node { width: percent(100), height: percent(100), min_height: px(0) }
            )]
        }
    }
}
