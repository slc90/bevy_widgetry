use crate::{
    field,
    popup::{self, ComboBoxPopup},
};
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListSelectionChanged,
    WidgetryListSelectionState, WidgetryListView, WidgetryListViewRenderer,
};
use std::collections::VecDeque;

#[derive(SceneComponent, FromTemplate)]
#[scene(WidgetryComboBoxProps<T>)]
#[require(crate::style::StyleInvalidation, ComboDiagnostics, ForwardedChanges, crate::colors::ColorState = crate::colors::ColorState::new::<T>())]
pub struct WidgetryComboBox<T: Send + Sync + 'static> {
    source: Entity,
    item_height: f32,
    max_visible_items: usize,
    renderer: WidgetryListViewRenderer<T>,
}

#[derive(Component, Default)]
pub(crate) struct ComboDiagnostics {
    pub(crate) projection: FailureState,
    pub(crate) icon: FailureState,
    initialization: FailureState,
}

pub struct WidgetryComboBoxProps<T: Send + Sync + 'static> {
    pub colors: crate::WidgetryComboBoxColorOverrides,
    pub source: Entity,
    pub item_height: f32,
    pub max_visible_items: usize,
    pub renderer: WidgetryListViewRenderer<T>,
}

#[derive(Component)]
struct ValidatedConfig;

#[derive(Component)]
pub(crate) struct Initialized;

#[derive(Component)]
pub(crate) struct PreservePopup;

#[derive(Component, Default)]
pub(crate) struct ForwardedChanges(VecDeque<ValueChange<Option<WidgetryListItemId>>>);

pub(crate) fn initialize_selection<T: Send + Sync + 'static>(
    mut roots: Query<
        (
            Entity,
            &WidgetryComboBox<T>,
            &Children,
            &mut ComboDiagnostics,
        ),
        Without<Initialized>,
    >,
    popups: Query<&Children, With<ComboBoxPopup>>,
    lists: Query<&WidgetryListSelectionState, With<WidgetryListView<T>>>,
    models: Query<&WidgetryListModel<T>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for (root, combo, children, mut diagnostics) in &mut roots {
        let result = (|| -> Result<(), BevyError> {
            let list = children.iter().find_map(|popup| {
                popups
                    .get(popup)
                    .ok()?
                    .iter()
                    .find(|&child| lists.contains(child))
            });
            let Some(list) = list else {
                return Err(BevyError::error("ComboBox internal ListView missing"));
            };
            let Ok(model) = models.get(combo.source) else {
                // The internal ListView already validates and propagates missing-source errors.
                // Skip initialization here to avoid diagnosing the same failure twice.
                return Ok(());
            };
            if lists
                .get(list)
                .is_ok_and(|state| state.selected_id().is_none())
                && !model.is_empty()
            {
                project_selection::<T>(&mut commands, list, 0);
            }
            commands.entity(root).insert(Initialized);

            Ok(())
        })();
        let result = diagnostics.initialization.observe(
            result,
            |error| widgetry_error!(?root, %error, "ComboBox selection 初始化失败"),
            || widgetry_info!(?root, "ComboBox selection 初始化恢复正常"),
        );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}

pub(crate) fn handle_value_change<T: Send + Sync + 'static>(
    event: On<WidgetryListSelectionChanged>,
    lists: Query<(&ChildOf, Has<PreservePopup>), With<WidgetryListView<T>>>,
    mut popups: Query<(&ChildOf, &mut Visibility), With<ComboBoxPopup>>,
    mut roots: Query<(Has<InteractionDisabled>, &mut ForwardedChanges), With<WidgetryComboBox<T>>>,
    focus: Option<ResMut<InputFocus>>,
    mut commands: Commands,
) {
    let value = event.selected.iter().next().copied();
    let Ok((list_parent, preserve)) = lists.get(event.source) else {
        return;
    };
    let Ok((popup_parent, mut visibility)) = popups.get_mut(list_parent.parent()) else {
        return;
    };
    let root = popup_parent.parent();
    let Ok((disabled, mut changes)) = roots.get_mut(root) else {
        return;
    };
    if !disabled && !preserve && value.is_some() {
        *visibility = Visibility::Hidden;
        // Input for this frame has already been dispatched to the old target.
        // Release focus immediately so ListView rejects the remaining queued keyboard operations.
        if let Some(mut focus) = focus
            && focus.get() == Some(event.source)
        {
            focus.clear();
        }
    }
    changes.0.push_back(ValueChange {
        source: root,
        value,
        is_final: true,
    });
    commands.queue(move |world: &mut World| dispatch_value_changes(world, root));
}

// Other observers of the internal event may queue the next programmatic request first.
// Dispatch the old payload first so notification order matches selection commit order.
fn dispatch_value_changes(world: &mut World, root: Entity) {
    while let Some(event) = world
        .get_mut::<ForwardedChanges>(root)
        .and_then(|mut changes| changes.0.pop_front())
    {
        world.trigger(event);
    }
}

// Observers of the previous notification may queue model mutations.
// Queue dispatch separately so those commands run before the next request resolves a stable ID from a potentially stale index.
fn queue_pending_changes<T: Send + Sync + 'static>(commands: &mut Commands, root: Entity) {
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryComboBox<T>>(root).is_some() {
            dispatch_value_changes(world, root);
        }
    });
}

fn project_selection<T: Send + Sync + 'static>(
    commands: &mut Commands,
    list: Entity,
    index: usize,
) {
    commands.queue(move |world: &mut World| -> Result<(), BevyError> {
        let id = world
            .get::<WidgetryListView<T>>(list)
            .and_then(|view| world.get::<WidgetryListModel<T>>(view.source()))
            .and_then(|model| model.id(index));
        let Some(id) = id else {
            widgetry_error!(?list, index, "ComboBox 内部 selection projection 目标失效");
            return Err(BevyError::error(
                "ComboBox internal selection projection target missing",
            ));
        };
        bevy_widgetry_list_view::internal::set_selection_id_silent::<T, WidgetryListItemId>(
            world,
            list,
            Some(id),
        )?;
        Ok(())
    });
}

fn update_error(root: Entity, reason: &str) -> BevyError {
    widgetry_error!(?root, reason, "ComboBox state 更新失败");
    BevyError::error(format!("ComboBox state 更新失败: {reason}"))
}

fn update_list<T: Send + Sync + 'static>(
    world: &World,
    root: Entity,
) -> Result<(Entity, Entity), BevyError> {
    let combo = world
        .get::<WidgetryComboBox<T>>(root)
        .ok_or_else(|| update_error(root, "Widget 不存在或 type 不匹配"))?;
    let source = combo.source();
    if world.get::<WidgetryListModel<T>>(source).is_none() {
        return Err(update_error(root, "source 不存在或 Model type 不匹配"));
    }
    let list = world
        .get::<Children>(root)
        .and_then(|children| {
            children.iter().find_map(|popup| {
                world.get::<ComboBoxPopup>(popup)?;
                world.get::<Visibility>(popup)?;
                world
                    .get::<Children>(popup)?
                    .iter()
                    .find(|&child| world.get::<WidgetryListView<T>>(child).is_some())
            })
        })
        .ok_or_else(|| update_error(root, "内部 ListView 缺失"))?;
    if world
        .get::<WidgetryListView<T>>(list)
        .is_none_or(|view| view.source() != source)
        || world.get::<WidgetryListSelectionState>(list).is_none()
    {
        return Err(update_error(root, "内部 ListView source 或 state 无效"));
    }
    Ok((list, source))
}

fn queue_selection<T: Send + Sync + 'static>(
    world: &mut World,
    list: Entity,
    index: Option<usize>,
) {
    let already_preserved = world.get::<PreservePopup>(list).is_some();
    world.entity_mut(list).insert(PreservePopup);
    match index {
        Some(index) => WidgetryListView::<T>::select_index(&mut world.commands(), list, index),
        None => WidgetryListView::<T>::clear_selection(&mut world.commands(), list),
    }
    if !already_preserved {
        world.commands().queue(move |world: &mut World| {
            if let Ok(mut entity) = world.get_entity_mut(list) {
                entity.remove::<PreservePopup>();
            }
        });
    }
}

impl<T: Send + Sync + 'static> WidgetryComboBox<T> {
    pub fn source(&self) -> Entity {
        self.source
    }

    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    pub fn max_visible_items(&self) -> usize {
        self.max_visible_items
    }

    pub fn renderer(&self) -> &WidgetryListViewRenderer<T> {
        &self.renderer
    }

    pub fn set_selected(commands: &mut Commands, entity: Entity, item_id: WidgetryListItemId) {
        queue_pending_changes::<T>(commands, entity);
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (list, source) = update_list::<T>(world, entity)?;
            let index = world
                .get::<WidgetryListModel<T>>(source)
                .and_then(|model| model.index_of(item_id))
                .ok_or_else(|| update_error(entity, "id 不存在于当前 source"))?;
            queue_selection::<T>(world, list, Some(index));
            Ok(())
        });
    }

    pub fn clear_selection(commands: &mut Commands, entity: Entity) {
        queue_pending_changes::<T>(commands, entity);
        commands.queue(move |world: &mut World| -> Result<(), BevyError> {
            let (list, _) = update_list::<T>(world, entity)?;
            queue_selection::<T>(world, list, None);
            Ok(())
        });
    }

    fn scene(props: WidgetryComboBoxProps<T>) -> impl Scene {
        let height = props.item_height * props.max_visible_items as f32 + 2.0;
        let max_visible_items = props.max_visible_items;
        let source = props.source;
        let popup = popup::scene::<T>(props.source, props.item_height, props.renderer.clone());
        bsn! {
            template(move |_| props.colors.clone().initial::<T>())
            WidgetryComboBox::<T> {
                source: {props.source}, item_height: {props.item_height},
                max_visible_items: {props.max_visible_items}, renderer: {props.renderer},
            }
            template(move |_| {
                if max_visible_items == 0 || !height.is_finite() {
                    widgetry_error!(source = ?source, max_visible_items = max_visible_items, "ComboBox viewport 高度必须有限且行数非零");
                    return Err(bevy_widgetry_core::scene::logged_error("WidgetryComboBox requires a finite viewport and nonzero max_visible_items"));
                }
                Ok(ValidatedConfig)
            })
            Node { width: px(200) }
            Children [@field::scene()-- @{popup}]
        }
    }
}

impl<T: Send + Sync + 'static> Default for WidgetryComboBoxProps<T> {
    fn default() -> Self {
        Self {
            colors: default(),
            source: Entity::PLACEHOLDER,
            item_height: 32.0,
            max_visible_items: 8,
            renderer: WidgetryListViewRenderer::default(),
        }
    }
}
