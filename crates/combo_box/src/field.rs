use crate::combo_box::{ComboDiagnostics, WidgetryComboBox};
use crate::popup::ComboBoxPopup;
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::scene::apply_scene;
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListView, WidgetryListViewState,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxField;

#[derive(Component, Default, Clone)]
#[require(FieldProjection)]
pub(crate) struct ComboBoxFieldContent;

#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxDropdownIcon;

#[derive(Component, Default)]
struct FieldProjection(Option<RenderedSelection>);

#[derive(Clone, Copy, PartialEq, Eq)]
struct RenderedSelection {
    id: WidgetryListItemId,
    index: usize,
    revision: u64,
}

pub(crate) fn project<T: Send + Sync + 'static>(world: &mut World) -> Result<(), BevyError> {
    let roots = world
        .query::<(Entity, &WidgetryComboBox<T>, &Children)>()
        .iter(world)
        .map(|(root, combo, children)| {
            (
                root,
                combo.source(),
                combo.renderer().clone(),
                children.iter().collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    let mut failure = None;
    for (root, source, renderer, children) in roots {
        let result = (|| -> Result<(), BevyError> {
            let content = children.iter().find_map(|&field| {
                world.get::<ComboBoxField>(field)?;
                world
                    .get::<Children>(field)?
                    .iter()
                    .find(|&child| world.get::<ComboBoxFieldContent>(child).is_some())
            });
            let content = projection_invariant(content, root)?;
            let list = children.iter().find_map(|&popup| {
                world.get::<ComboBoxPopup>(popup)?;
                world
                    .get::<Children>(popup)?
                    .iter()
                    .find(|&child| world.get::<WidgetryListView<T>>(child).is_some())
            });
            let list = projection_invariant(list, root)?;
            let selected =
                projection_invariant(world.get::<WidgetryListViewState>(list), root)?.selected;
            let model = projection_invariant(world.get::<WidgetryListModel<T>>(source), root)?;
            let projection = selected
                .map(|id| -> Result<_, BevyError> {
                    let index = projection_invariant(model.index_of(id), root)?;
                    Ok(RenderedSelection {
                        id,
                        index,
                        revision: projection_invariant(model.revision(index), root)?,
                    })
                })
                .transpose()?;
            if projection_invariant(world.get::<FieldProjection>(content), root)?.0 == projection {
                return Ok(());
            }
            let scene = projection
                .map(|selected| -> Result<_, BevyError> {
                    renderer.render(
                        selected.index,
                        projection_invariant(model.get(selected.index), root)?,
                    )
                })
                .transpose()?;
            let old_children = world
                .get::<Children>(content)
                .map(|children| children.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            world.entity_mut(content).insert(FieldProjection(None));
            for child in old_children {
                world.despawn(child);
            }
            if let Some(scene) = scene
                && let Err(error) =
                    apply_scene(&mut world.entity_mut(content), bsn! { Children [{scene}] })
            {
                // 失败时 cache 为 None；清理已成功的部分 child，避免后续空 selection 跳过清理。
                let partial = world
                    .get::<Children>(content)
                    .map(|children| children.to_vec())
                    .unwrap_or_default();
                for child in partial {
                    world.despawn(child);
                }
                return Err(BevyError::error(error));
            }
            world
                .entity_mut(content)
                .insert(FieldProjection(projection));

            Ok(())
        })();
        let result = if let Some(mut diagnostics) = world.get_mut::<ComboDiagnostics>(root) {
            diagnostics.projection.observe(
                result,
                |error| widgetry_error!(?root, %error, "ComboBox Field projection 失败"),
                || widgetry_info!(?root, "ComboBox Field projection 恢复正常"),
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

fn projection_invariant<T>(value: Option<T>, _root: Entity) -> Result<T, BevyError> {
    value.ok_or_else(|| BevyError::error("ComboBox Field projection invariant failed"))
}

pub(crate) fn scene() -> impl Scene {
    bsn! {
        ComboBoxField
        @WidgetryButton
        Node {
            width: percent(100), height: px(36),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(10), px(0)),
            border: UiRect::all(px(1)),
        }
        Children [
            (ComboBoxFieldContent Node { align_items: AlignItems::Center }),
            (ComboBoxDropdownIcon
                @WidgetryIcon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::new(16, 16))} }
                Node { width: px(16), height: px(16), flex_shrink: 0.0 }),
        ]
    }
}

pub(crate) fn mirror_disabled_added<T: Send + Sync + 'static>(
    roots: Query<&Children, (With<WidgetryComboBox<T>>, Added<InteractionDisabled>)>,
    fields: Query<(), With<ComboBoxField>>,
    mut popups: Query<&mut Visibility, With<ComboBoxPopup>>,
    mut commands: Commands,
) {
    for children in &roots {
        for child in children.iter() {
            if fields.contains(child) {
                commands.entity(child).insert(InteractionDisabled);
            }
            if let Ok(mut visibility) = popups.get_mut(child) {
                *visibility = Visibility::Hidden;
            }
        }
    }
}

// 同帧 remove 后重新 insert disabled 时仍会收到旧 Remove；只查询当前 enabled root，避免旧通知错误恢复 Field 输入。
pub(crate) fn mirror_disabled_removed<T: Send + Sync + 'static>(
    mut removed: RemovedComponents<InteractionDisabled>,
    roots: Query<&Children, (With<WidgetryComboBox<T>>, Without<InteractionDisabled>)>,
    fields: Query<(), With<ComboBoxField>>,
    mut commands: Commands,
) {
    for root in removed.read() {
        if let Ok(children) = roots.get(root) {
            for child in children.iter().filter(|&child| fields.contains(child)) {
                commands.entity(child).remove::<InteractionDisabled>();
            }
        }
    }
}

pub(crate) fn initialize_disabled<T: Send + Sync + 'static>(
    fields: Query<(Entity, &ChildOf), Added<ComboBoxField>>,
    roots: Query<Has<InteractionDisabled>, With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    for (field, parent) in &fields {
        let Ok(disabled) = roots.get(parent.parent()) else {
            continue;
        };
        if disabled {
            commands.entity(field).insert(InteractionDisabled);
        } else {
            commands.entity(field).remove::<InteractionDisabled>();
        }
    }
}

// root 刚禁用时下一次 PreUpdate 尚未同步内部 ListView；在 Add observer 中排队 mirror，避免此间输入仍能改选。
pub(crate) fn on_disabled_added<T: Send + Sync + 'static>(
    event: On<Add, InteractionDisabled>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        queue_list_disabled::<T>(&mut commands, event.entity);
    }
}

// Remove observer 执行时仍能读到待移除 component，且同帧可能再次 insert；延后读取最终 root state，避免错误解除内部 disabled。
pub(crate) fn on_disabled_removed<T: Send + Sync + 'static>(
    event: On<Remove, InteractionDisabled>,
    roots: Query<(), With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    if roots.contains(event.entity) {
        queue_list_disabled::<T>(&mut commands, event.entity);
    }
}

fn queue_list_disabled<T: Send + Sync + 'static>(commands: &mut Commands, root: Entity) {
    commands.queue(move |world: &mut World| {
        if world.get::<WidgetryComboBox<T>>(root).is_none() {
            return;
        }
        let disabled = world.get::<InteractionDisabled>(root).is_some();
        let lists = world
            .get::<Children>(root)
            .into_iter()
            .flat_map(|children| children.iter())
            .filter(|&popup| world.get::<ComboBoxPopup>(popup).is_some())
            .flat_map(|popup| {
                world
                    .get::<Children>(popup)
                    .into_iter()
                    .flat_map(|children| children.iter())
            })
            .filter(|&list| world.get::<WidgetryListView<T>>(list).is_some())
            .collect::<Vec<_>>();
        for list in lists {
            if disabled {
                world.entity_mut(list).insert(InteractionDisabled);
            } else {
                world.entity_mut(list).remove::<InteractionDisabled>();
            }
        }
    });
}

pub(crate) fn mirror_list_disabled<T: Send + Sync + 'static>(
    lists: Query<(Entity, &ChildOf, Has<InteractionDisabled>), With<WidgetryListView<T>>>,
    popups: Query<&ChildOf, With<ComboBoxPopup>>,
    roots: Query<Has<InteractionDisabled>, With<WidgetryComboBox<T>>>,
    mut commands: Commands,
) {
    for (list, parent, list_disabled) in &lists {
        let Ok(popup_parent) = popups.get(parent.parent()) else {
            continue;
        };
        let Ok(disabled) = roots.get(popup_parent.parent()) else {
            continue;
        };
        if disabled != list_disabled {
            if disabled {
                commands.entity(list).insert(InteractionDisabled);
            } else {
                commands.entity(list).remove::<InteractionDisabled>();
            }
        }
    }
}

pub(crate) fn sync_icon<T: Send + Sync + 'static>(
    popups: Query<(&ChildOf, &Visibility), (With<ComboBoxPopup>, Changed<Visibility>)>,
    mut roots: Query<(&Children, &mut ComboDiagnostics), With<WidgetryComboBox<T>>>,
    fields: Query<&Children, With<ComboBoxField>>,
    icons: Query<(), (With<WidgetryIcon>, With<ComboBoxDropdownIcon>)>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut failure = None;
    for (parent, visibility) in &popups {
        let root = parent.parent();
        let Ok((children, mut diagnostics)) = roots.get_mut(root) else {
            continue;
        };
        let result = (|| -> Result<(), BevyError> {
            let icon = children.iter().find_map(|field| {
                fields
                    .get(field)
                    .ok()?
                    .iter()
                    .find(|&child| icons.contains(child))
            });
            let Some(icon) = icon else {
                return Err(BevyError::error("ComboBox dropdown icon missing"));
            };
            let path = if *visibility == Visibility::Visible {
                BuiltinIcon::ChevronUp
            } else {
                BuiltinIcon::ChevronDown
            };
            WidgetryIcon::set_svg(&mut commands, icon, path.path());

            Ok(())
        })();
        let result = diagnostics.icon.observe(
            result,
            |error| widgetry_error!(?root, %error, "ComboBox dropdown icon 失效"),
            || widgetry_info!(?root, "ComboBox dropdown icon 恢复正常"),
        );
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(error);
        }
    }
    failure.map_or(Ok(()), Err)
}
