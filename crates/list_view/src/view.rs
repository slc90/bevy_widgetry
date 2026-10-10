use crate::behavior::ListNavigation;
use crate::{WidgetryListItemId, WidgetryListModel};
use bevy::a11y::AccessibilityNode;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::prelude::*;
use bevy::ui_widgets::ActiveDescendant;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use bevy_widgetry_scroll_area::{
    ScrollAxis, ScrollbarPolicy, ScrollbarVisibility, WidgetryScrollArea,
};
use std::sync::Arc;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryListViewProps<T>)]
#[require(WidgetryListViewState, ListNavigation, ListDiagnostics, crate::colors::ColorState = crate::colors::ColorState::new::<T>())]
pub struct WidgetryListView<T: Send + Sync + 'static> {
    source: Entity,
    item_height: f32,
    renderer: WidgetryListViewRenderer<T>,
}

#[derive(Component, Default)]
pub(crate) struct ListDiagnostics {
    source: FailureState,
    pub(crate) runtime: FailureState,
}

pub struct WidgetryListViewProps<T: Send + Sync + 'static> {
    pub colors: crate::WidgetryListViewColorOverrides,
    pub source: Entity,
    pub item_height: f32,
    pub renderer: WidgetryListViewRenderer<T>,
}

pub struct WidgetryListViewRenderer<T>(
    Option<Arc<dyn Fn(usize, &T) -> Box<dyn SceneList> + Send + Sync>>,
);

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[component(immutable)]
pub struct WidgetryListViewState {
    pub selected: Option<WidgetryListItemId>,
    pub active: Option<WidgetryListItemId>,
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetryListViewItem {
    pub id: WidgetryListItemId,
    pub index: usize,
}

#[derive(Component, Default, Clone)]
pub(crate) struct TopSpacer;

#[derive(Component, Default, Clone)]
pub(crate) struct BottomSpacer;

pub(crate) fn validate_sources<T: Send + Sync + 'static>(
    world: &mut World,
) -> Result<(), BevyError> {
    let roots = world
        .query_filtered::<Entity, With<WidgetryListView<T>>>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut failure = None;
    for root in roots {
        if let Err(error) = validate_source::<T>(world, root)
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn validate_source<T: Send + Sync + 'static>(
    world: &mut World,
    root: Entity,
) -> Result<(), BevyError> {
    let source = world
        .get::<WidgetryListView<T>>(root)
        .map(WidgetryListView::source)
        .unwrap_or(Entity::PLACEHOLDER);
    let result = if world.get::<WidgetryListModel<T>>(source).is_some() {
        Ok(())
    } else {
        Err(BevyError::error(
            "WidgetryListView requires a live matching WidgetryListModel source",
        ))
    };
    let Some(mut diagnostics) = world.get_mut::<ListDiagnostics>(root) else {
        return result;
    };
    diagnostics.source.observe(result,
        |error| widgetry_error!(entity = ?root, ?source, item_type = std::any::type_name::<T>(), %error, "ListView source 不存在或缺少匹配的 ListModel"),
        || widgetry_info!(entity = ?root, ?source, "ListView source 恢复正常"))
}

impl<T: Send + Sync + 'static> WidgetryListView<T> {
    pub fn set_selected(commands: &mut Commands, list: Entity, index: usize) {
        commands
            .queue(move |world: &mut World| crate::behavior::set_selected::<T>(world, list, index));
    }

    pub fn clear_selection(commands: &mut Commands, list: Entity) {
        commands.queue(move |world: &mut World| crate::behavior::clear_selection::<T>(world, list));
    }

    pub fn set_active(commands: &mut Commands, list: Entity, index: Option<usize>) {
        commands
            .queue(move |world: &mut World| crate::behavior::set_active::<T>(world, list, index));
    }

    pub fn source(&self) -> Entity {
        self.source
    }

    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    pub fn renderer(&self) -> &WidgetryListViewRenderer<T> {
        &self.renderer
    }

    fn scene(props: WidgetryListViewProps<T>) -> impl Scene {
        let source = props.source;
        let item_height = props.item_height;
        let renderer_missing = props.renderer.0.is_none();
        bsn! {
            bevy_widgetry_core::foreground::ResolvedForeground
            BackgroundColor
            template(move |_| props.colors.clone().initial::<T>())
            template(move |_| {
                if source == Entity::PLACEHOLDER || renderer_missing {
                    widgetry_error!(source = ?source, "ListView 构造必须提供 source 和 renderer");
                    return Err(bevy_widgetry_core::scene::logged_error("WidgetryListView requires source and renderer"));
                }
                if !item_height.is_finite() || item_height <= 0.0 {
                    widgetry_error!(source = ?source, item_height = item_height, "ListView item_height 必须为有限正数");
                    return Err(bevy_widgetry_core::scene::logged_error("WidgetryListView requires a finite positive item_height"));
                }
                Ok(ListNavigation::default())
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
            TabIndex::default()
            ActiveDescendant::default()
            template(|_| Ok(AccessibilityNode(accesskit::Node::new(accesskit::Role::ListBox))))
            BorderColor::default()
            Node {
                min_width: px(0), min_height: px(0), border: UiRect::all(px(1)),
                border_radius: BorderRadius::all(px(4)), overflow: Overflow::clip(),
            }
            WidgetryListView::<T> {
                source: {props.source},
                item_height: {props.item_height},
                renderer: {props.renderer},
            }
        }
    }
}

impl<T: Send + Sync + 'static> Default for WidgetryListViewProps<T> {
    fn default() -> Self {
        Self {
            colors: default(),
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            renderer: WidgetryListViewRenderer(None),
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
        F: Fn(usize, &T) -> S + Send + Sync + 'static,
    {
        Self(Some(Arc::new(move |index, value| {
            Box::new(factory(index, value))
        })))
    }

    pub fn render(&self, index: usize, value: &T) -> Result<Box<dyn SceneList>, BevyError> {
        let Some(factory) = &self.0 else {
            widgetry_error!(index, "ListView renderer 缺失");
            return Err(BevyError::error("WidgetryListView requires renderer"));
        };
        Ok(factory(index, value))
    }
}
