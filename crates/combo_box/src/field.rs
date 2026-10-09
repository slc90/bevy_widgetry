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

pub(crate) fn own_dropdown_icon(
    event: On<Add, ComboBoxDropdownIcon>,
    parents: Query<&ChildOf>,
    fields: Query<(), With<ComboBoxField>>,
    mut commands: Commands,
) {
    if let Ok(parent) = parents.get(event.entity)
        && fields.contains(parent.parent())
    {
        commands
            .entity(event.entity)
            .insert(
                bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryIcon>::new(parent.parent()),
            );
    }
}

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
                // 失败时 cache 为 None。
                // 清理已成功的部分 child，避免后续空 selection 跳过清理。
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

pub(crate) fn on_disabled_added<T: Send + Sync + 'static>(
    event: On<Add, InteractionDisabled>,
    roots: Query<&Children, With<WidgetryComboBox<T>>>,
    mut popups: Query<&mut Visibility, With<ComboBoxPopup>>,
) {
    if let Ok(children) = roots.get(event.entity) {
        for child in children.iter() {
            if let Ok(mut visibility) = popups.get_mut(child) {
                *visibility = Visibility::Hidden;
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
