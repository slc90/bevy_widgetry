const VIEW_CORNER_RADIUS: f32 = 4.0;

use crate::WidgetryListItemId;
use crate::WidgetryListKey;
use crate::behavior::ListNavigation;
use crate::error::{RuntimeFault, StyleFault};
use crate::source::DataSource;
use crate::{
    ListConfigurationRejection, ListError, ListFailure, ListSourceRejection,
    error::{failure, invalid_source, invalid_view},
};
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{FailureState, WidgetryEcsError},
};

use bevy_widgetry_scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
};
use std::sync::Arc;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryListViewProps<T, K>)]
#[require(crate::WidgetryListSelectionState<K>, WidgetryListViewState<K>, ListNavigation<K>, ListDiagnostics<K>, crate::style::StyleInvalidation, crate::colors::ColorState = crate::colors::ColorState::new::<T, K>())]
pub struct WidgetryListView<T: Send + Sync + 'static, K: WidgetryListKey = WidgetryListItemId> {
    source: Entity,
    item_height: f32,
    selection_mode: crate::WidgetryListSelectionMode,
    draggable: bool,
    renderer: WidgetryListViewRenderer<T>,
    projection: Option<crate::WidgetryListProjectionPolicy<K>>,
}

#[derive(Component)]
pub(crate) struct ListDiagnostics<K: WidgetryListKey> {
    source: FailureState<(Entity, ListSourceRejection)>,
    pub(crate) runtime: FailureState<RuntimeFault<K>>,
    pub(crate) style: FailureState<StyleFault>,
}
impl<K: WidgetryListKey> Default for ListDiagnostics<K> {
    fn default() -> Self {
        Self {
            source: default(),
            runtime: default(),
            style: default(),
        }
    }
}

pub struct WidgetryListViewProps<T: Send + Sync + 'static, K: WidgetryListKey = WidgetryListItemId>
{
    pub colors: crate::WidgetryListViewColorOverrides,
    pub source: Entity,
    pub item_height: f32,
    pub selection_mode: crate::WidgetryListSelectionMode,
    pub draggable: bool,
    pub renderer: WidgetryListViewRenderer<T>,
    pub projection: Option<crate::WidgetryListProjectionPolicy<K>>,
}

pub struct WidgetryListViewRenderer<T>(Option<Arc<dyn Fn(&T) -> Box<dyn SceneList> + Send + Sync>>);

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
#[component(immutable)]
pub struct WidgetryListViewState<K: WidgetryListKey = WidgetryListItemId> {
    pub active: Option<K>,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryListViewItem<K: WidgetryListKey = WidgetryListItemId> {
    pub id: K,
    pub index: usize,
}

#[derive(Component, Default, Clone)]
pub(crate) struct TopSpacer;

#[derive(Component, Default, Clone)]
pub(crate) struct BottomSpacer;

pub(crate) fn validate_sources<T: Send + Sync + 'static, K: WidgetryListKey>(world: &mut World) {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T, K>>>()
        .iter(world)
        .collect::<Vec<_>>();
    for root in roots {
        if let Err(error) = validate_source::<T, K>(world, root) {
            world
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> {
                    Err(error.into_bevy_error())
                });
        }
    }
}

pub(crate) fn validate_source<T: Send + Sync + 'static, K: WidgetryListKey>(
    world: &mut World,
    root: Entity,
) -> Result<(), WidgetryEcsError> {
    let Some(view) = world.get::<WidgetryListView<T, K>>(root) else {
        return Ok(());
    };
    let source = view.source();
    let reason = match world.get_resource::<DataSource<T, K>>() {
        None => Some(ListSourceRejection::Unregistered),
        Some(adapter) if (adapter.changed_at)(world, source).is_none() => {
            Some(ListSourceRejection::Unavailable)
        }
        Some(_) => None,
    };
    let Some(reason) = reason else {
        if let Some(mut diagnostics) = world.get_mut::<ListDiagnostics<K>>(root) {
            diagnostics.source.recover(|| {
                ErrorFrame::new("list_view.validate_source")
                    .field("view", root)
                    .field("source", source)
            });
        }
        return Ok(());
    };
    let create = || -> Arc<dyn bevy_widgetry_diagnostics::bevy::Diagnostic> {
        Arc::new(
            invalid_source::<T, K>(root, source, reason, "list_view.validate_source").with_context(
                ErrorFrame::new("list_view.validate_source")
                    .field("view", root)
                    .field("source", source),
            ),
        )
    };
    Err(match world.get_mut::<ListDiagnostics<K>>(root) {
        Some(mut diagnostics) => diagnostics.source.failure((source, reason), create),
        None => WidgetryEcsError::from_shared(create()),
    })
}

impl<K: WidgetryListKey> Default for WidgetryListViewState<K> {
    fn default() -> Self {
        Self { active: None }
    }
}

impl<T: Send + Sync + 'static, K: WidgetryListKey> WidgetryListView<T, K> {
    pub(crate) fn projection_policy(&self) -> Option<&crate::WidgetryListProjectionPolicy<K>> {
        self.projection.as_ref()
    }

    pub fn set_projection_in_world(
        world: &mut World,
        root: Entity,
        policy: Option<crate::WidgetryListProjectionPolicy<K>>,
    ) -> Result<(), ListFailure> {
        let Some(mut view) = world.get_mut::<Self>(root) else {
            return Err(invalid_view::<T, K>(root, "list_view.configure"));
        };
        view.projection = policy;
        world
            .entity_mut(root)
            .remove::<crate::projection::ViewOrder<K>>();
        world
            .entity_mut(root)
            .remove::<crate::WidgetryListProjection<K>>();
        crate::drag::cancel::<K>(world, root);
        Ok(())
    }

    pub fn set_projection(
        commands: &mut Commands,
        root: Entity,
        policy: Option<crate::WidgetryListProjectionPolicy<K>>,
    ) {
        commands.queue(move |world: &mut World| {
            Self::set_projection_in_world(world, root, policy).map_err(ListFailure::into_bevy_error)
        });
    }

    pub fn clear_selection(commands: &mut Commands, list: Entity) {
        commands.queue(move |world: &mut World| {
            Self::clear_selection_in_world(world, list)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }

    pub fn set_active_display_index(commands: &mut Commands, list: Entity, index: Option<usize>) {
        commands.queue(move |world: &mut World| {
            crate::behavior::set_active::<T, K>(world, list, index)
                .map_err(ListFailure::into_bevy_error)
        });
    }

    pub fn selection_mode(&self) -> crate::WidgetryListSelectionMode {
        self.selection_mode
    }

    pub fn source(&self) -> Entity {
        self.source
    }

    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    pub fn draggable(&self) -> bool {
        self.draggable
    }

    pub fn set_draggable(commands: &mut Commands, root: Entity, enabled: bool) {
        commands.queue(move |world: &mut World| {
            Self::set_draggable_in_world(world, root, enabled)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }

    pub fn set_draggable_in_world(
        world: &mut World,
        root: Entity,
        enabled: bool,
    ) -> Result<bool, ListFailure> {
        if world.get::<Self>(root).is_none() {
            return Err(invalid_view::<T, K>(root, "list_view.configure"));
        }
        let Some(mut view) = world.get_mut::<Self>(root) else {
            return Ok(false);
        };
        if view.draggable == enabled {
            return Ok(false);
        }
        view.draggable = enabled;
        if !enabled {
            crate::drag::cancel::<K>(world, root);
        }
        Ok(true)
    }

    pub fn reset_view_order_in_world(world: &mut World, root: Entity) -> Result<bool, ListFailure> {
        let Some(_) = world.get::<Self>(root) else {
            return Err(invalid_view::<T, K>(root, "list_view.configure"));
        };
        let changed = world
            .entity_mut(root)
            .take::<crate::projection::ViewOrder<K>>()
            .is_some();
        if changed {
            world
                .entity_mut(root)
                .remove::<crate::WidgetryListProjection<K>>();
            crate::drag::cancel::<K>(world, root);
        }
        Ok(changed)
    }

    pub fn reset_view_order(commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| {
            Self::reset_view_order_in_world(world, root)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }

    pub fn set_source_in_world(
        world: &mut World,
        root: Entity,
        source: Entity,
    ) -> Result<bool, ListFailure> {
        if world.get::<Self>(root).is_none() {
            return Err(invalid_view::<T, K>(root, "list_view.set_source"));
        }
        let adapter = world.get_resource::<DataSource<T, K>>().ok_or_else(|| {
            invalid_source::<T, K>(
                root,
                source,
                ListSourceRejection::Unregistered,
                "list_view.set_source",
            )
        })?;
        if (adapter.changed_at)(world, source).is_none() {
            return Err(invalid_source::<T, K>(
                root,
                source,
                ListSourceRejection::Unavailable,
                "list_view.set_source",
            ));
        }
        let Some(mut view) = world.get_mut::<Self>(root) else {
            return Ok(false);
        };
        if view.source == source {
            return Ok(false);
        }
        view.source = source;
        // IDs and revisions are source-local; old rows also own in-flight Pointer hits.
        crate::drag::cancel::<K>(world, root);
        crate::virtualization::clear_rows(world, root);
        world
            .entity_mut(root)
            .remove::<crate::projection::ViewOrder<K>>();
        world
            .entity_mut(root)
            .remove::<crate::WidgetryListProjection<K>>();
        crate::drag::cancel::<K>(world, root);
        Ok(true)
    }

    pub fn set_source(commands: &mut Commands, root: Entity, source: Entity) {
        commands.queue(move |world: &mut World| {
            Self::set_source_in_world(world, root, source)
                .map(|_| ())
                .map_err(ListFailure::into_bevy_error)
        });
    }

    pub fn renderer(&self) -> &WidgetryListViewRenderer<T> {
        &self.renderer
    }

    fn scene(props: WidgetryListViewProps<T, K>) -> impl Scene {
        let source = props.source;
        let item_height = props.item_height;
        let renderer_missing = props.renderer.0.is_none();
        bsn! {
            bevy_widgetry_core::foreground::ResolvedForeground
            BackgroundColor
            template(move |context| props.colors.clone().initial::<T,K>(context.entity.id()).map_err(ListFailure::into_bevy_error))
            template(move |context| {
                if source == Entity::PLACEHOLDER || renderer_missing {
                    return Err(failure(ListError::Configuration {view:context.entity.id(),source,item_height,renderer_missing,reason:if source == Entity::PLACEHOLDER {ListConfigurationRejection::MissingSource} else {ListConfigurationRejection::MissingRenderer}}, "list_view.initialize").into_bevy_error());
                }
                if !item_height.is_finite() || item_height <= 0.0 {
                    return Err(failure(ListError::Configuration {view:context.entity.id(),source,item_height,renderer_missing,reason:ListConfigurationRejection::InvalidItemHeight}, "list_view.initialize").into_bevy_error());
                }
                Ok(ListNavigation::<K>::default())
            })
            @WidgetryScrollArea {
                @axis: ScrollAxis::Vertical,
                @scrollbar_visibility: {ScrollbarVisibility { horizontal: ScrollbarPolicy::Hidden, vertical: ScrollbarPolicy::Hidden }},
                @keyboard_scroll: false,
                @children: bsn_list!{
                    TopSpacer Node { height: px(0), flex_shrink: 0.0 } template(|_| Ok(Pickable::IGNORE))--
                    BottomSpacer Node { height: px(0), flex_shrink: 0.0 } template(|_| Ok(Pickable::IGNORE))
                },
            }
            TabIndex(-1)
            BorderColor::default()
            Node {
                min_width: px(0), min_height: px(0), border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(VIEW_CORNER_RADIUS)), overflow: Overflow::clip(),
            }
            WidgetryListView::<T, K> {
                source: {props.source},
                item_height: {props.item_height},
                selection_mode: {props.selection_mode},
                draggable: {props.draggable},
                renderer: {props.renderer},
                projection: {props.projection},
            }
        }
    }
}

impl<T: Send + Sync + 'static, K: WidgetryListKey> Default for WidgetryListViewProps<T, K> {
    fn default() -> Self {
        Self {
            colors: default(),
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            selection_mode: default(),
            draggable: false,
            renderer: WidgetryListViewRenderer(None),
            projection: None,
        }
    }
}

impl<T> Clone for WidgetryListViewRenderer<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for WidgetryListViewRenderer<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T> WidgetryListViewRenderer<T> {
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn(&T) -> S + Send + Sync + 'static,
    {
        Self(Some(Arc::new(move |value| Box::new(factory(value)))))
    }

    pub fn render(&self, value: &T) -> Result<Box<dyn SceneList>, ListFailure> {
        let Some(factory) = &self.0 else {
            return Err(failure(
                ListError::MissingRenderer {
                    item_type: std::any::type_name::<T>(),
                },
                "list_view.render",
            ));
        };
        Ok(factory(value))
    }
}

pub fn source_reference<T: Send + Sync + 'static, K: WidgetryListKey>(
    source: bevy::ecs::template::EntityTemplate,
) -> impl Scene {
    bsn! { WidgetryListView::<T, K> { source: {source} } }
}
