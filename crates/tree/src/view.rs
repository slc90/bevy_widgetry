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
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewRenderer,
    WidgetryListViewState,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryTreeViewProps)]
#[require(ViewDiagnostics, crate::colors::ColorState)]
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

#[derive(Component, Default)]
struct ExpanderDiagnostics(FailureState);

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

pub(crate) fn validate_hierarchy(world: &mut World) -> Result<(), BevyError> {
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
            internal_list(world, root)?;
            Ok(())
        })();
        let result = if let Some(mut diagnostics) = world.get_mut::<ViewDiagnostics>(root) {
            diagnostics.hierarchy.observe(
                result,
                |error| widgetry_error!(?root, %error, "TreeView hierarchy 失效"),
                || widgetry_info!(?root, "TreeView hierarchy 恢复正常"),
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

pub(crate) fn establish_style_owners(world: &mut World) -> Result<(), BevyError> {
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    let roots = world
        .query_filtered::<Entity, With<WidgetryTreeView>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let list = internal_list(world, root)?;
        if world
            .get::<WidgetryStyleOwner<WidgetryListView<WidgetryTreeVisibleItem>>>(list)
            .is_none()
        {
            world.entity_mut(list).insert(WidgetryStyleOwner::<
                WidgetryListView<WidgetryTreeVisibleItem>,
            >::new(root));
        }
    }
    let buttons = world
        .query_filtered::<Entity, With<TreeExpander>>()
        .iter(world)
        .collect::<Vec<_>>();
    for button in buttons {
        let root = tree_root(world, button).ok_or_else(|| {
            widgetry_error!(?button, "Tree expander 缺失颜色 owner");
            BevyError::error("Tree expander 缺失颜色 owner")
        })?;
        if world
            .get::<WidgetryStyleOwner<WidgetryButton>>(button)
            .is_none()
        {
            world
                .entity_mut(button)
                .insert(WidgetryStyleOwner::<WidgetryButton>::new(root));
        }
    }
    Ok(())
}
pub(crate) fn update_colors(world: &mut World) -> Result<(), BevyError> {
    use bevy::picking::hover::Hovered;
    use bevy::ui::Pressed;
    use bevy_widgetry_core::color::WidgetryStyleOwner;
    let theme = world
        .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
        .colors()
        .tree;
    let roots = world
        .query_filtered::<Entity, With<WidgetryTreeView>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        let colors = crate::WidgetryTreeColorOverrides::get(world, root)?.resolve(&theme);
        let list = internal_list(world, root)?;
        bevy_widgetry_list_view::internal::apply_owned_list_colors::<WidgetryTreeVisibleItem>(
            world,
            list,
            &bevy_widgetry_theme::WidgetryListViewColors {
                container: colors.container,
                item: colors.item,
            },
        )?;
    }
    let buttons = world
        .query::<(Entity, &TreeExpander, &WidgetryStyleOwner<WidgetryButton>)>()
        .iter(world)
        .map(|(button, expander, owner)| (button, expander.source, expander.node, owner.entity))
        .collect::<Vec<_>>();
    for (button, source, node, root) in buttons {
        let colors = crate::WidgetryTreeColorOverrides::get(world, root)?.resolve(&theme);
        let expanded = world
            .get::<WidgetryTreeModel>(source)
            .is_some_and(|model| model.state().is_expanded(node));
        let colors = if expanded {
            colors.expander.expanded
        } else {
            colors.expander.collapsed
        };
        let state = if world.get::<InteractionDisabled>(button).is_some() {
            colors.disabled
        } else if world.get::<Pressed>(button).is_some() {
            colors.pressed
        } else if world.get::<Hovered>(button).is_some_and(|h| h.0) {
            colors.hovered
        } else {
            colors.normal
        };
        bevy_widgetry_button::internal::apply_owned_button_colors(
            world,
            button,
            bevy_widgetry_theme::WidgetryButtonStateColors {
                background: state.background,
                border: state.border,
                foreground: state.foreground,
            },
        )?;
    }
    Ok(())
}
pub(crate) fn refresh_colors(
    _event: On<bevy_widgetry_theme::WidgetryThemeChanged>,
    mut commands: Commands,
) {
    commands.queue(update_colors);
}

pub(crate) fn own_list(
    event: On<Add, WidgetryListView<WidgetryTreeVisibleItem>>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryTreeView>>,
    mut commands: Commands,
) {
    if let Ok(parent) = parents.get(event.entity)
        && roots.contains(parent.parent())
    {
        commands
            .entity(event.entity)
            .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<
                WidgetryListView<WidgetryTreeVisibleItem>,
            >::new(parent.parent()));
    }
}
pub(crate) fn own_expander(
    event: On<Add, TreeExpander>,
    parents: Query<&ChildOf>,
    roots: Query<(), With<WidgetryTreeView>>,
    mut commands: Commands,
) {
    if let Some(root) = parents
        .iter_ancestors(event.entity)
        .find(|root| roots.contains(*root))
    {
        commands
            .entity(event.entity)
            .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<
                WidgetryButton,
            >::new(root));
    }
}
