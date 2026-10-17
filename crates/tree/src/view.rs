use crate::renderer::TreeContent;
use crate::{WidgetryTreeModel, WidgetryTreeVisibleItem};
use bevy::asset::AssetPath;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::{Activate, ValueChange};
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};

pub(crate) mod style;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTreeViewProps)]
#[require(ViewDiagnostics, crate::colors::ColorState, style::TreeStyle)]
pub struct WidgetryTreeView {
    source: Entity,
}

#[derive(Component, Default)]
struct ViewDiagnostics {
    source: FailureState,
    selection: FailureState,
    hierarchy: FailureState,
}

pub struct WidgetryTreeViewProps {
    pub colors: crate::WidgetryTreeColorOverrides,
    pub source: Entity,
    pub item_height: f32,
    pub indent_width: f32,
    pub icons: WidgetryTreeIcons,
}

#[derive(Clone)]
pub struct WidgetryTreeIcons {
    pub expand: AssetPath<'static>,
    pub collapse: AssetPath<'static>,
}

#[derive(Component)]
struct ValidatedConfig;

#[derive(Component, Clone, Copy)]
#[require(ExpanderDiagnostics)]
pub(crate) struct TreeExpander {
    source: Entity,
    node: Entity,
}

#[cfg(test)]
thread_local! {
    static OWNER_WORK: std::cell::RefCell<Vec<Entity>> = const { std::cell::RefCell::new(Vec::new()) };
}

#[derive(Component, Default)]
struct ExpanderDiagnostics(FailureState);

#[cfg(test)]
thread_local! {
    static DIAGNOSTIC_WORK: std::cell::RefCell<Vec<Entity>> = const { std::cell::RefCell::new(Vec::new()) };
}

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
            let mut state = *invariant(world.get::<WidgetryListViewState>(list), root)?;
            if state.selected != selected {
                state.selected = selected;
                if selected.is_some() {
                    state.active = selected;
                }
                world.entity_mut(list).insert(state);
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

pub(crate) fn on_selection(
    event: On<ValueChange<Option<WidgetryListItemId>>>,
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
    let Some(id) = event.value else {
        return;
    };
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        if world.get::<InteractionDisabled>(root).is_some() {
            return Ok(());
        }
        let node = world
            .get::<WidgetryListModel<WidgetryTreeVisibleItem>>(source)
            .and_then(|model| model.get_by_id(id))
            .map(|item| item.entity);
        if let Some(node) = node {
            WidgetryTreeModel::select(world, source, Some(node))?;
        }
        Ok(())
    });
}

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

fn invariant<T>(value: Option<T>, _root: Entity) -> Result<T, BevyError> {
    value.ok_or_else(|| BevyError::error("WidgetryTreeView invariant failed"))
}

fn tree_root(world: &World, mut entity: Entity) -> Option<Entity> {
    loop {
        if world.get::<WidgetryTreeView>(entity).is_some() {
            return Some(entity);
        }
        entity = world.get::<ChildOf>(entity)?.parent();
    }
}

fn validate_view_hierarchy(world: &mut World, root: Entity) -> Result<(), BevyError> {
    let result = internal_list(world, root).map(|_| ());
    if let Some(mut diagnostics) = world.get_mut::<ViewDiagnostics>(root) {
        diagnostics.hierarchy.observe(
            result,
            |error| widgetry_error!(?root, %error, "TreeView hierarchy 失效"),
            || widgetry_info!(?root, "TreeView hierarchy 恢复正常"),
        )
    } else {
        result
    }
}

fn validate_expander_hierarchy(world: &mut World, button: Entity) -> Result<(), BevyError> {
    let result = if tree_root(world, button).is_some() {
        Ok(())
    } else {
        Err(BevyError::error("Tree expander has no owning TreeView"))
    };
    observe_expander_hierarchy(world, button, result)
}

fn observe_expander_hierarchy(
    world: &mut World,
    button: Entity,
    result: Result<(), BevyError>,
) -> Result<(), BevyError> {
    if let Some(mut diagnostics) = world.get_mut::<ExpanderDiagnostics>(button) {
        diagnostics.0.observe(
            result,
            |error| widgetry_error!(?button, %error, "Tree expander ownership 失效"),
            || widgetry_info!(?button, "Tree expander ownership 恢复正常"),
        )
    } else {
        result
    }
}

pub(crate) fn validate_changed(world: &mut World) -> Result<(), BevyError> {
    let mut pending = std::mem::take(&mut world.resource_mut::<StructureChanges>().diagnostics)
        .into_iter()
        .collect::<Vec<_>>();
    let mut visited = bevy::ecs::entity::EntityHashSet::default();
    let mut roots = bevy::ecs::entity::EntityHashSet::default();
    let mut buttons = bevy::ecs::entity::EntityHashSet::default();
    while let Some((entity, change)) = pending.pop() {
        if !world.entities().contains(entity) {
            continue;
        }
        if change == OwnerChange::Subtree
            && visited.insert(entity)
            && let Some(children) = world.get::<Children>(entity)
        {
            pending.extend(
                children
                    .iter()
                    .filter(|child| world.get::<WidgetryTreeView>(*child).is_none())
                    .map(|child| (child, OwnerChange::Subtree)),
            );
        }
        if (world.get::<WidgetryTreeView>(entity).is_some()
            || world
                .get::<WidgetryListView<WidgetryTreeVisibleItem>>(entity)
                .is_some()
            || world.get::<TreeExpander>(entity).is_some()
            || world.get::<OwnedPart>(entity).is_some())
            && let Some(root) = tree_root(world, entity)
        {
            roots.insert(root);
        }
        if world.get::<TreeExpander>(entity).is_some() {
            buttons.insert(entity);
        }
    }
    let mut failure = None;
    for root in roots {
        #[cfg(test)]
        DIAGNOSTIC_WORK.with_borrow_mut(|work| work.push(root));
        for result in [
            validate_source(world, root),
            validate_view_hierarchy(world, root),
        ] {
            if let Err(error) = result
                && failure.is_none()
            {
                failure = Some(error);
            }
        }
    }
    for button in buttons {
        #[cfg(test)]
        DIAGNOSTIC_WORK.with_borrow_mut(|work| work.push(button));
        if let Err(error) = validate_expander_hierarchy(world, button)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
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
            colors: default(),
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            indent_width: 20.0,
            icons: WidgetryTreeIcons::default(),
        }
    }
}

impl WidgetryTreeView {
    pub fn source(&self) -> Entity {
        self.source
    }

    fn scene(props: WidgetryTreeViewProps) -> impl Scene {
        let source = props.source;
        let indent = props.indent_width;
        let icons = props.icons;
        bsn! {
            template(move |_| props.colors.clone().initial())
            WidgetryTreeView { source }
            template(move |_| {
                if source == Entity::PLACEHOLDER || !indent.is_finite() || indent < 0.0 {
                    widgetry_error!(source = ?source, indent_width = indent, "TreeView 必须提供 source 与有限非负 indent_width");
                    return Err(bevy_widgetry_core::scene::logged_error("Invalid WidgetryTreeView configuration"));
                }
                Ok(ValidatedConfig)
            })
            Node { min_width: px(0), min_height: px(0), flex_direction: FlexDirection::Column }
            Children [
                @WidgetryListView::<WidgetryTreeVisibleItem> {
                    @source: source,
                    @item_height: {props.item_height},
                    @renderer: {WidgetryListViewRenderer::new(move |_, item: &WidgetryTreeVisibleItem| {
                        let node = item.entity;
                        let padding = f32::from(item.depth) * indent;
                        let icon = if item.expanded { icons.collapse.clone() } else { icons.expand.clone() };
                        let visibility = if item.has_children { Visibility::Inherited } else { Visibility::Hidden };
                        bsn_list!{
                            template(move |_| {
                                if !padding.is_finite() {
                                    return Err(BevyError::error("TreeView indentation overflow"));
                                }
                                Ok(Node { width: percent(100), align_items: AlignItems::Center, padding: UiRect::left(px(padding)), column_gap: px(6), ..default() })
                            })
                            Children [
                                @WidgetryButton { @children: {Some(Box::new(bsn_list! {
                                    @WidgetryIcon { @path: icon, @max_size: {Some(UVec2::splat(12))} }
                                    template(|_| Ok(Pickable::IGNORE))
                                    Node { width: px(12), height: px(12) }
                                }) as Box<dyn bevy::prelude::SceneList>)}, }
                                template(move |_| Ok(TreeExpander { source, node }))
                                template(move |_| Ok(visibility))
                                Node { width: px(20), height: px(20), min_height: px(20), padding: UiRect::ZERO, flex_shrink: 0.0, align_items: AlignItems::Center, justify_content: JustifyContent::Center }
                                on(on_expand)

                            -- template(move |_| Ok(TreeContent { node })) Node { min_width: px(0), flex_grow: 1.0 }]
                        }
                    })},
                }
                Node { width: percent(100), height: percent(100), min_height: px(0) }
            ]
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct StructureChanges {
    owners: bevy::ecs::entity::EntityHashMap<OwnerChange>,
    diagnostics: bevy::ecs::entity::EntityHashMap<OwnerChange>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OwnerChange {
    Part,
    Subtree,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PartKind {
    List,
    Expander,
}

#[derive(Component)]
struct OwnedPart {
    root: Entity,
    kind: PartKind,
}

pub(crate) fn install_style_owners(app: &mut App) {
    app.init_resource::<StructureChanges>()
        .add_observer(own_list)
        .add_observer(own_expander)
        .add_observer(owner_structure_added)
        .add_observer(owner_structure_removed)
        .add_observer(owner_parent_inserted)
        .add_observer(owner_parent_discarded)
        .add_observer(list_owner_discarded)
        .add_observer(expander_owner_discarded)
        .add_observer(source_inserted)
        .add_observer(source_removed);
    let world = app.world_mut();
    // Plugin installation may follow Scene creation, so compensate once before relying on lifecycle signals.
    let pending = world
        .query_filtered::<Entity, Or<(
            With<WidgetryTreeView>,
            With<WidgetryListView<WidgetryTreeVisibleItem>>,
            With<TreeExpander>,
        )>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut changes = world.resource_mut::<StructureChanges>();
    for entity in pending {
        queue_structure(&mut changes, entity, OwnerChange::Subtree);
    }
}

fn queue_structure(changes: &mut StructureChanges, entity: Entity, change: OwnerChange) {
    for pending in [&mut changes.owners, &mut changes.diagnostics] {
        pending
            .entry(entity)
            .and_modify(|current| {
                if change == OwnerChange::Subtree {
                    *current = change;
                }
            })
            .or_insert(change);
    }
}

fn queue_source(
    source: Entity,
    views: &Query<(Entity, &WidgetryTreeView)>,
    changes: &mut StructureChanges,
) {
    for (root, view) in views {
        if view.source == source {
            changes.diagnostics.entry(root).or_insert(OwnerChange::Part);
        }
    }
}

fn source_inserted(
    event: On<Insert<WidgetryTreeModel>>,
    views: Query<(Entity, &WidgetryTreeView)>,
    mut changes: ResMut<StructureChanges>,
) {
    queue_source(event.entity, &views, &mut changes);
}

fn source_removed(
    event: On<Remove<WidgetryTreeModel>>,
    views: Query<(Entity, &WidgetryTreeView)>,
    mut changes: ResMut<StructureChanges>,
) {
    queue_source(event.entity, &views, &mut changes);
}

fn owner_structure_added(event: On<Add<WidgetryTreeView>>, mut changes: ResMut<StructureChanges>) {
    queue_structure(&mut changes, event.entity, OwnerChange::Subtree);
}

fn owner_structure_removed(
    event: On<
        Remove<(
            WidgetryTreeView,
            WidgetryListView<WidgetryTreeVisibleItem>,
            TreeExpander,
        )>,
    >,
    mut changes: ResMut<StructureChanges>,
) {
    queue_structure(&mut changes, event.entity, OwnerChange::Subtree);
}

fn queue_owner_parent(
    entity: Entity,
    parts: &Query<
        (),
        Or<(
            With<WidgetryTreeView>,
            With<WidgetryListView<WidgetryTreeVisibleItem>>,
            With<TreeExpander>,
            With<OwnedPart>,
        )>,
    >,
    roots: &Query<(), With<WidgetryTreeView>>,
    parents: &Query<&ChildOf>,
    changes: &mut StructureChanges,
) {
    if parts.contains(entity)
        || parents
            .iter_ancestors(entity)
            .any(|parent| roots.contains(parent))
    {
        queue_structure(changes, entity, OwnerChange::Subtree);
        if parts.contains(entity)
            && let Some(root) = parents
                .iter_ancestors(entity)
                .find(|parent| roots.contains(*parent))
        {
            changes.diagnostics.entry(root).or_insert(OwnerChange::Part);
        }
    }
}

fn owner_parent_inserted(
    event: On<Insert<ChildOf>>,
    parts: Query<
        (),
        Or<(
            With<WidgetryTreeView>,
            With<WidgetryListView<WidgetryTreeVisibleItem>>,
            With<TreeExpander>,
            With<OwnedPart>,
        )>,
    >,
    roots: Query<(), With<WidgetryTreeView>>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<StructureChanges>,
) {
    queue_owner_parent(event.entity, &parts, &roots, &parents, &mut changes);
}

fn owner_parent_discarded(
    event: On<Discard<ChildOf>>,
    parts: Query<
        (),
        Or<(
            With<WidgetryTreeView>,
            With<WidgetryListView<WidgetryTreeVisibleItem>>,
            With<TreeExpander>,
            With<OwnedPart>,
        )>,
    >,
    roots: Query<(), With<WidgetryTreeView>>,
    parents: Query<&ChildOf>,
    mut changes: ResMut<StructureChanges>,
) {
    queue_owner_parent(event.entity, &parts, &roots, &parents, &mut changes);
}

fn list_owner_discarded(
    event: On<
        Discard<
            bevy_widgetry_core::color::WidgetryStyleOwner<
                WidgetryListView<WidgetryTreeVisibleItem>,
            >,
        >,
    >,
    parts: Query<(), With<OwnedPart>>,
    mut changes: ResMut<StructureChanges>,
) {
    if parts.contains(event.entity) {
        changes
            .owners
            .entry(event.entity)
            .or_insert(OwnerChange::Part);
    }
}

fn expander_owner_discarded(
    event: On<Discard<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryButton>>>,
    parts: Query<(), With<OwnedPart>>,
    mut changes: ResMut<StructureChanges>,
) {
    if parts.contains(event.entity) {
        changes
            .owners
            .entry(event.entity)
            .or_insert(OwnerChange::Part);
    }
}

fn release_part(world: &mut World, entity: Entity) {
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    let Some(part) = world.get::<OwnedPart>(entity) else {
        return;
    };
    let (root, kind) = (part.root, part.kind);
    world.entity_mut(entity).remove::<OwnedPart>();
    match kind {
        PartKind::List => {
            if world
                .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>(entity)
                .is_some_and(|owner| owner.entity == root)
            {
                world
                    .entity_mut(entity)
                    .remove::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>();
            }
        }
        PartKind::Expander => {
            if world
                .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                .is_some_and(|owner| owner.entity == root)
            {
                world
                    .entity_mut(entity)
                    .remove::<WidgetryStyleOwner<WidgetryButton>>();
            }
        }
    }
}

pub(crate) fn establish_style_owners(world: &mut World) -> Result<(), BevyError> {
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    let mut pending = std::mem::take(&mut world.resource_mut::<StructureChanges>().owners)
        .into_iter()
        .collect::<Vec<_>>();
    let mut visited = bevy::ecs::entity::EntityHashSet::default();
    let mut visited_subtrees = bevy::ecs::entity::EntityHashSet::default();
    let mut failure = None;
    while let Some((entity, subtree)) = pending.pop() {
        if !world.entities().contains(entity) {
            continue;
        }
        if subtree == OwnerChange::Subtree
            && visited_subtrees.insert(entity)
            && let Some(children) = world.get::<Children>(entity)
        {
            // Nested TreeViews maintain their own parts, independent of an outer TreeView's reparenting.
            pending.extend(
                children
                    .iter()
                    .filter(|child| world.get::<WidgetryTreeView>(*child).is_none())
                    .map(|child| (child, OwnerChange::Subtree)),
            );
        }
        if !visited.insert(entity) {
            continue;
        }
        let owner = if world
            .get::<WidgetryListView<WidgetryTreeVisibleItem>>(entity)
            .is_some()
        {
            world
                .get::<ChildOf>(entity)
                .map(ChildOf::parent)
                .filter(|root| world.get::<WidgetryTreeView>(*root).is_some())
                .map(|root| (root, PartKind::List))
        } else if world.get::<TreeExpander>(entity).is_some() {
            let root = tree_root(world, entity);
            let result = root
                .map(|_| ())
                .ok_or_else(|| BevyError::error("Tree expander has no owning TreeView"));
            if let Err(error) = observe_expander_hierarchy(world, entity, result)
                && failure.is_none()
            {
                failure = Some(error);
            }
            root.map(|root| (root, PartKind::Expander))
        } else {
            None
        };
        let Some((root, kind)) = owner else {
            release_part(world, entity);
            continue;
        };
        #[cfg(test)]
        OWNER_WORK.with_borrow_mut(|work| work.push(entity));
        match kind {
            PartKind::List => {
                if world
                    .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>(entity)
                    .is_none_or(|owner| owner.entity != root)
                {
                    world.entity_mut(entity).insert(WidgetryStyleOwner::<
                        WidgetryListView<WidgetryTreeVisibleItem>,
                    >::new(root));
                }
            }
            PartKind::Expander => {
                if world
                    .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                    .is_none_or(|owner| owner.entity != root)
                {
                    world
                        .entity_mut(entity)
                        .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
                }
            }
        }
        if world
            .get::<OwnedPart>(entity)
            .is_none_or(|part| part.root != root || part.kind != kind)
        {
            world.entity_mut(entity).insert(OwnedPart { root, kind });
        }
    }
    failure.map_or(Ok(()), Err)
}
pub(crate) fn own_list(
    event: On<Add<WidgetryListView<WidgetryTreeVisibleItem>>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryTreeView>>,
    mut commands: Commands,
    mut changes: ResMut<StructureChanges>,
) {
    queue_structure(&mut changes, event.entity, OwnerChange::Part);
    if let Ok(parent) = parents.get(event.entity)
        && roots.contains(parent.parent())
    {
        commands
            .entity(event.entity)
            .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<
                WidgetryListView<WidgetryTreeVisibleItem>,
            >::new(parent.parent()))
            .insert(OwnedPart {
                root: parent.parent(),
                kind: PartKind::List,
            });
    }
}
pub(crate) fn own_expander(
    event: On<Add<TreeExpander>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryTreeView>>,
    mut commands: Commands,
    mut changes: ResMut<StructureChanges>,
) {
    queue_structure(&mut changes, event.entity, OwnerChange::Part);
    if let Some(root) = parents
        .iter_ancestors(event.entity)
        .find(|root| roots.contains(*root))
    {
        commands
            .entity(event.entity)
            .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<
                WidgetryButton,
            >::new(root))
            .insert(OwnedPart {
                root,
                kind: PartKind::Expander,
            });
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{WidgetryTreeAppExt, WidgetryTreeNode, WidgetryTreePlugin, WidgetryTreeRenderer};
    use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
    use bevy_widgetry_test_utils::scene_app;

    #[derive(Component)]
    struct Label;

    fn fixture() -> (App, Entity, Vec<Entity>) {
        let mut app = scene_app();
        app.init_resource::<UiScale>()
            .add_plugins(WidgetryTreePlugin);
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, _| {
            bsn_list! { Text("node") bevy_widgetry_core::text::WidgetryText }
        }))
        .unwrap();
        let hierarchy = app.world_mut().spawn_empty().id();
        for _ in 0..3 {
            app.world_mut()
                .spawn((WidgetryTreeNode, Label, ChildOf(hierarchy)));
        }
        let source = app
            .world_mut()
            .spawn(WidgetryTreeModel::new(hierarchy))
            .id();
        let mut roots = Vec::new();
        for _ in 0..2 {
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
                .unwrap()
                .id();
            let list = internal_list(app.world(), root).unwrap();
            let viewport = app
                .world()
                .get::<Children>(list)
                .unwrap()
                .iter()
                .find(|child| {
                    app.world()
                        .get::<WidgetryScrollAreaViewport>(*child)
                        .is_some()
                })
                .unwrap();
            app.world_mut().entity_mut(viewport).insert(ComputedNode {
                size: Vec2::new(400.0, 96.0),
                inverse_scale_factor: 1.0,
                ..default()
            });
            roots.push(root);
        }
        app.update();
        app.update();
        (app, source, roots)
    }

    #[test]
    fn stable_tree_views_skip_palette_and_expander_color_work() {
        let (mut app, _, _) = fixture();
        style::COLOR_WORK.with_borrow_mut(|work| {
            work.0.clear();
            work.1.clear();
        });
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            style::COLOR_WORK.with_borrow(|work| (work.0.len(), work.1.len())),
            (0, 0)
        );
    }

    #[derive(Resource, Default)]
    struct HoverRequest(Option<Entity>);

    #[test]
    fn expander_interaction_and_removal_only_recolor_the_affected_button() {
        use bevy::picking::hover::Hovered;
        use bevy::ui::Pressed;
        let (mut app, _, roots) = fixture();
        let button = app
            .world_mut()
            .query_filtered::<Entity, With<TreeExpander>>()
            .iter(app.world())
            .find(|entity| tree_root(app.world(), *entity) == Some(roots[0]))
            .unwrap();
        app.init_resource::<HoverRequest>()
            .add_systems(Update, |world: &mut World| {
                if let Some(button) = world.resource_mut::<HoverRequest>().0.take() {
                    world.entity_mut(button).insert(Hovered(true));
                }
            });
        let check = |app: &mut App, color| {
            style::COLOR_WORK.with_borrow_mut(|work| {
                work.0.clear();
                work.1.clear();
            });
            app.update();
            assert_eq!(app.world().get::<BackgroundColor>(button).unwrap().0, color);
            style::COLOR_WORK.with_borrow(|work| {
                assert!(work.0.is_empty());
                assert_eq!(work.1, vec![button]);
            });
        };
        let colors = bevy_widgetry_theme::WidgetryThemeMode::Dark
            .colors()
            .tree
            .expander
            .collapsed;
        app.world_mut().resource_mut::<HoverRequest>().0 = Some(button);
        check(&mut app, colors.hovered.background);
        app.world_mut().entity_mut(button).remove::<Hovered>();
        check(&mut app, colors.normal.background);
        app.world_mut().entity_mut(button).insert(Pressed);
        check(&mut app, colors.pressed.background);
        app.world_mut().entity_mut(button).remove::<Pressed>();
        check(&mut app, colors.normal.background);
        app.world_mut()
            .entity_mut(button)
            .insert(InteractionDisabled);
        check(&mut app, colors.disabled.background);
        app.world_mut()
            .entity_mut(button)
            .remove::<InteractionDisabled>();
        check(&mut app, colors.normal.background);
    }

    #[test]
    fn palette_changes_stay_in_their_view_and_missing_outputs_retry_until_repaired() {
        use bevy_widgetry_core::foreground::ResolvedForeground;
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
        LogCapture::default().run(|| {
            let (mut app, _, roots) = fixture();
            let mut colors = crate::WidgetryTreeColorOverrides::default();
            colors.expander.collapsed.normal.foreground = Some(Color::srgb_u8(17, 34, 51));
            style::COLOR_WORK.with_borrow_mut(|work| {
                work.0.clear();
                work.1.clear();
            });
            crate::WidgetryTreeColorOverrides::set_in_world(app.world_mut(), roots[0], colors)
                .unwrap();
            app.update();
            let buttons = style::COLOR_WORK.with_borrow(|work| {
                assert_eq!(work.0, vec![roots[0]]);
                assert_eq!(work.1.len(), 3);
                work.1.clone()
            });
            for button in &buttons {
                assert_eq!(tree_root(app.world(), *button), Some(roots[0]));
                assert_eq!(
                    app.world().get::<ResolvedForeground>(*button).unwrap().0,
                    Color::srgb_u8(17, 34, 51)
                );
            }
            let button = buttons[0];
            let saved = app
                .world_mut()
                .entity_mut(button)
                .take::<BorderColor>()
                .unwrap();
            app.set_error_handler(ErrorCapture::handler());
            app.edit_schedule(PreUpdate, |schedule| {
                schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
            });
            let errors = ErrorCapture::default();
            for _ in 0..2 {
                update_with_errors(&mut app, &errors, true);
            }
            app.world_mut().entity_mut(button).insert(saved);
            update_with_errors(&mut app, &errors, false);
            style::COLOR_WORK.with_borrow_mut(|work| {
                work.0.clear();
                work.1.clear();
            });
            update_with_errors(&mut app, &errors, false);
            assert_eq!(
                style::COLOR_WORK.with_borrow(|work| (work.0.len(), work.1.len())),
                (0, 0)
            );
            crate::WidgetryTreeColorOverrides::clear_in_world(app.world_mut(), roots[0]).unwrap();
            app.update();
            assert_eq!(
                app.world().get::<ResolvedForeground>(button).unwrap().0,
                bevy_widgetry_theme::WidgetryThemeMode::Dark
                    .colors()
                    .tree
                    .expander
                    .collapsed
                    .normal
                    .foreground
            );
        });
    }

    #[test]
    fn idle_tree_views_do_not_rediscover_list_and_expander_owners() {
        let (mut app, _, _) = fixture();
        assert!(
            app.world_mut()
                .query_filtered::<Entity, With<TreeExpander>>()
                .iter(app.world())
                .count()
                >= 4
        );
        OWNER_WORK.with_borrow_mut(Vec::clear);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(OWNER_WORK.with_borrow(Vec::len), 0);
    }

    #[test]
    fn stable_views_skip_extra_source_and_hierarchy_diagnostics() {
        let (mut app, _, _) = fixture();
        app.edit_schedule(PreUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        DIAGNOSTIC_WORK.with_borrow_mut(Vec::clear);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(DIAGNOSTIC_WORK.with_borrow(Vec::len), 0);
    }

    fn assert_owners(world: &mut World) {
        use bevy_widgetry_core::color::WidgetryStyleOwner;
        let parts = world
            .query::<(Entity, &OwnedPart)>()
            .iter(world)
            .map(|(entity, part)| (entity, part.root, part.kind))
            .collect::<Vec<_>>();
        for (entity, root, kind) in parts {
            assert_eq!(tree_root(world, entity), Some(root));
            let owner = match kind {
                PartKind::List => {
                    world
                        .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>(
                            entity,
                        )
                        .unwrap()
                        .entity
                }
                PartKind::Expander => {
                    world
                        .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                        .unwrap()
                        .entity
                }
            };
            assert_eq!(owner, root);
        }
    }

    #[test]
    fn owner_repairs_remain_local_and_do_not_take_business_buttons() {
        use bevy_widgetry_core::color::WidgetryStyleOwner;
        let (mut app, _, roots) = fixture();
        assert_owners(app.world_mut());
        let list = internal_list(app.world(), roots[0]).unwrap();
        let expander = app
            .world_mut()
            .query_filtered::<Entity, With<TreeExpander>>()
            .iter(app.world())
            .find(|entity| tree_root(app.world(), *entity) == Some(roots[0]))
            .unwrap();
        OWNER_WORK.with_borrow_mut(Vec::clear);
        app.world_mut()
            .entity_mut(list)
            .remove::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>();
        app.world_mut()
            .entity_mut(expander)
            .remove::<WidgetryStyleOwner<WidgetryButton>>();
        establish_style_owners(app.world_mut()).unwrap();
        let work = OWNER_WORK.with_borrow_mut(std::mem::take);
        assert_eq!(work.len(), 2);
        assert!(work.contains(&list) && work.contains(&expander));
        assert_owners(app.world_mut());
        app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, _| bsn_list! {
            @WidgetryButton { @children: {Some(Box::new(bsn_list! {Text("business action") bevy_widgetry_core::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, }
        })).unwrap();
        app.update();
        let buttons = app
            .world_mut()
            .query_filtered::<Entity, (With<WidgetryButton>, Without<TreeExpander>)>()
            .iter(app.world())
            .collect::<Vec<_>>();
        assert!(!buttons.is_empty());
        for button in buttons {
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .is_none()
            );
        }
        assert_owners(app.world_mut());
        app.update();
        OWNER_WORK.with_borrow_mut(Vec::clear);
        app.world_mut().despawn(roots[0]);
        app.update();
        app.update();
        assert_eq!(OWNER_WORK.with_borrow(Vec::len), 0);
        assert!(app.world().resource::<StructureChanges>().owners.is_empty());
        assert_owners(app.world_mut());
    }

    #[test]
    fn delayed_subtrees_and_reparenting_follow_the_nearest_tree_view() {
        use bevy_widgetry_core::color::WidgetryStyleOwner;
        use bevy_widgetry_test_utils::LogCapture;
        let logs = LogCapture::default();
        logs.run(|| {
            let (mut app, source, roots) = fixture();
            let node = app
                .world()
                .get::<WidgetryTreeModel>(source)
                .unwrap()
                .visible_items()[0]
                .entity;
            let subtree = app.world_mut().spawn_scene(bsn! {
                Node Children [@WidgetryButton template(move |_| Ok(TreeExpander { source, node }))]
            }).unwrap().id();
            let button = app.world().get::<Children>(subtree).unwrap()[0];
            let error = establish_style_owners(app.world_mut()).unwrap_err();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            assert!(app.world().resource::<StructureChanges>().owners.is_empty());
            establish_style_owners(app.world_mut()).unwrap();
            app.world_mut().entity_mut(roots[1]).add_child(subtree);
            establish_style_owners(app.world_mut()).unwrap();
            assert_eq!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .unwrap()
                    .entity,
                roots[1]
            );
            app.world_mut().entity_mut(roots[0]).add_child(subtree);
            establish_style_owners(app.world_mut()).unwrap();
            assert_eq!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .unwrap()
                    .entity,
                roots[0]
            );
            app.world_mut().entity_mut(roots[0]).add_child(roots[1]);
            app.world_mut().entity_mut(roots[1]).add_child(subtree);
            establish_style_owners(app.world_mut()).unwrap();
            assert_owners(app.world_mut());
            assert_eq!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .unwrap()
                    .entity,
                roots[1]
            );
            let foreign = app.world_mut().spawn(Node::default()).id();
            app.world_mut().entity_mut(foreign).add_child(subtree);
            assert!(establish_style_owners(app.world_mut()).is_err());
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .is_none()
            );
            assert!(app.world().get::<OwnedPart>(button).is_none());
            assert!(app.world().resource::<StructureChanges>().owners.len() <= 1);
            establish_style_owners(app.world_mut()).unwrap();
            assert!(app.world().resource::<StructureChanges>().owners.is_empty());
            app.world_mut().entity_mut(roots[1]).add_child(subtree);
            establish_style_owners(app.world_mut()).unwrap();
            assert_owners(app.world_mut());
        });
        assert!(
            logs.records()
                .iter()
                .any(|record| record.level == bevy::log::Level::ERROR
                    && record.fields.contains_key("button"))
        );
    }
    #[test]
    fn marker_repairs_and_late_plugin_installation_restore_existing_parts() {
        use bevy_widgetry_core::color::WidgetryStyleOwner;
        use bevy_widgetry_list_view::{WidgetryListViewAppExt, WidgetryListViewPlugin};
        use bevy_widgetry_test_utils::LogCapture;
        let logs = LogCapture::default();
        logs.run(|| {
            let mut app = scene_app();
            app.init_resource::<UiScale>()
                .add_plugins(WidgetryListViewPlugin);
            app.register_widgetry_list_view::<WidgetryTreeVisibleItem>()
                .unwrap();
            let hierarchy = app.world_mut().spawn_empty().id();
            app.world_mut()
                .spawn((WidgetryTreeNode, Label, ChildOf(hierarchy)));
            let source = app
                .world_mut()
                .spawn(WidgetryTreeModel::new(hierarchy))
                .id();
            let root = app
                .world_mut()
                .spawn_scene(bsn! { @WidgetryTreeView { @source: source } })
                .unwrap()
                .id();
            let list = internal_list(app.world(), root).unwrap();
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>(list)
                    .is_none()
            );
            app.add_plugins(WidgetryTreePlugin);
            app.register_renderer::<Label>(WidgetryTreeRenderer::new(|_, _| bsn_list! { Node }))
                .unwrap();
            establish_style_owners(app.world_mut()).unwrap();
            assert_owners(app.world_mut());
            let button = app
                .world_mut()
                .spawn_scene(bsn! {
                    @WidgetryButton template(move |_| Ok(TreeExpander { source, node: hierarchy }))
                    template(move |_| Ok(ChildOf(root)))
                })
                .unwrap()
                .id();
            establish_style_owners(app.world_mut()).unwrap();
            let expander = *app.world().get::<TreeExpander>(button).unwrap();
            app.world_mut().entity_mut(button).remove::<TreeExpander>();
            establish_style_owners(app.world_mut()).unwrap();
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .is_none()
            );
            app.world_mut().entity_mut(button).insert(expander);
            establish_style_owners(app.world_mut()).unwrap();
            assert_eq!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .unwrap()
                    .entity,
                root
            );
            app.world_mut()
                .entity_mut(root)
                .remove::<WidgetryTreeView>();
            assert!(establish_style_owners(app.world_mut()).is_err());
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryButton>>(button)
                    .is_none()
            );
            assert!(
                app.world()
                    .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>(list)
                    .is_none()
            );
            app.world_mut()
                .entity_mut(root)
                .insert(WidgetryTreeView { source });
            establish_style_owners(app.world_mut()).unwrap();
            assert_owners(app.world_mut());
        });
    }
    fn update_with_errors(
        app: &mut App,
        capture: &bevy_widgetry_test_utils::ErrorCapture,
        fails: bool,
    ) {
        capture.run(|| app.update());
        let errors = capture.take();
        assert_eq!(!errors.is_empty(), fails);
        assert!(
            errors
                .iter()
                .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
        );
    }

    #[test]
    fn shared_source_failure_recovers_each_view_without_recovering_destroyed_views() {
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
        let logs = LogCapture::default();
        logs.run(|| {
            let (mut app, source, roots) = fixture();
            app.edit_schedule(PreUpdate, |schedule| {
                schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
            });
            app.set_error_handler(ErrorCapture::handler());
            let errors = ErrorCapture::default();
            let count = |level, message: &str| {
                logs.records()
                    .iter()
                    .filter(|record| {
                        record.level == level
                            && record
                                .fields
                                .get("message")
                                .is_some_and(|text| text.contains(message))
                    })
                    .count()
            };
            DIAGNOSTIC_WORK.with_borrow_mut(Vec::clear);
            let saved = app
                .world_mut()
                .entity_mut(source)
                .take::<WidgetryTreeModel>()
                .unwrap();
            for _ in 0..3 {
                update_with_errors(&mut app, &errors, true);
            }
            assert_eq!(DIAGNOSTIC_WORK.with_borrow(Vec::len), 2);
            assert_eq!(count(bevy::log::Level::ERROR, "source 缺失"), 2);
            assert_eq!(count(bevy::log::Level::INFO, "source 恢复"), 0);
            app.world_mut().entity_mut(source).insert(saved);
            update_with_errors(&mut app, &errors, false);
            assert_eq!(count(bevy::log::Level::INFO, "source 恢复"), 2);
            let saved = app
                .world_mut()
                .entity_mut(source)
                .take::<WidgetryTreeModel>()
                .unwrap();
            update_with_errors(&mut app, &errors, true);
            assert_eq!(count(bevy::log::Level::ERROR, "source 缺失"), 4);
            app.world_mut().despawn(roots[0]);
            update_with_errors(&mut app, &errors, true);
            assert_eq!(count(bevy::log::Level::INFO, "source 恢复"), 2);
            app.world_mut().entity_mut(source).insert(saved);
            update_with_errors(&mut app, &errors, false);
            assert_eq!(count(bevy::log::Level::INFO, "source 恢复"), 3);
            assert_owners(app.world_mut());
        });
    }

    #[test]
    fn hierarchy_failures_and_repairs_keep_view_diagnostics_independent() {
        use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
        let logs = LogCapture::default();
        logs.run(|| {
            let (mut app, _, roots) = fixture();
            app.edit_schedule(PreUpdate, |schedule| {
                schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
            });
            app.set_error_handler(ErrorCapture::handler());
            let errors = ErrorCapture::default();
            let list = internal_list(app.world(), roots[0]).unwrap();
            let saved = app
                .world_mut()
                .entity_mut(list)
                .take::<WidgetryListView<WidgetryTreeVisibleItem>>()
                .unwrap();
            for _ in 0..3 {
                update_with_errors(&mut app, &errors, true);
            }
            let count = |level, message: &str| {
                logs.records()
                    .iter()
                    .filter(|record| {
                        record.level == level
                            && record
                                .fields
                                .get("message")
                                .is_some_and(|text| text.contains(message))
                    })
                    .count()
            };
            assert_eq!(count(bevy::log::Level::ERROR, "hierarchy 失效"), 1);
            assert_eq!(
                count(bevy::log::Level::ERROR, "selection projection 失败"),
                1
            );
            assert_eq!(count(bevy::log::Level::INFO, "恢复正常"), 0);
            app.world_mut().entity_mut(list).insert(saved);
            update_with_errors(&mut app, &errors, false);
            assert_eq!(count(bevy::log::Level::INFO, "hierarchy 恢复正常"), 1);
            assert_eq!(
                count(bevy::log::Level::INFO, "selection projection 恢复正常"),
                1
            );
            assert_owners(app.world_mut());
            let expander = app
                .world_mut()
                .query_filtered::<Entity, With<TreeExpander>>()
                .iter(app.world())
                .find(|entity| tree_root(app.world(), *entity) == Some(roots[0]))
                .unwrap();
            let parent = app.world().get::<ChildOf>(expander).unwrap().parent();
            app.world_mut().entity_mut(expander).remove::<ChildOf>();
            update_with_errors(&mut app, &errors, true);
            assert_eq!(count(bevy::log::Level::ERROR, "expander ownership 失效"), 1);
            app.world_mut().entity_mut(expander).insert(ChildOf(parent));
            update_with_errors(&mut app, &errors, false);
            assert_eq!(
                count(bevy::log::Level::INFO, "expander ownership 恢复正常"),
                1
            );
            for record in logs.records().iter().filter(|record| {
                record.level == bevy::log::Level::INFO
                    && record
                        .fields
                        .get("message")
                        .is_some_and(|text| text.contains("恢复正常"))
            }) {
                assert_ne!(record.fields.get("root"), Some(&format!("{:?}", roots[1])));
            }
        });
    }
}
