use crate::combo_box::{ComboDiagnostics, WidgetryComboBox};
use crate::popup::ComboBoxPopup;
use crate::{
    ComboPart,
    error::{self, ComboFault, RuntimeFailure, part},
};
use bevy::prelude::*;
use bevy_widgetry_asset::BuiltinIcon;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::icon::WidgetryIcon;
use bevy_widgetry_core::scene::{SceneFailure, attempt::apply_scene};
use bevy_widgetry_diagnostics::bevy::{DiagnosticBatch, WidgetryEcsError};
use bevy_widgetry_list_view::{
    WidgetryListItemId, WidgetryListModel, WidgetryListSelectionState, WidgetryListView,
};

#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxField;

#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxButtonContent;

#[derive(Component, Default, Clone)]
#[require(FieldProjection)]
pub(crate) struct ComboBoxFieldContent;

#[derive(Component, Default, Clone)]
pub(crate) struct ComboBoxDropdownIcon;

pub(crate) fn own_dropdown_icon(
    event: On<Add<ComboBoxDropdownIcon>>,
    parents: Query<&ChildOf>,
    fields: Query<(), With<ComboBoxField>>,
    mut commands: Commands,
) {
    let mut ancestor = parents.get(event.entity).ok().map(ChildOf::parent);
    while let Some(entity) = ancestor {
        if fields.contains(entity) {
            commands
                .entity(event.entity)
                .insert(bevy_widgetry_core::color::WidgetryStyleOwner::<WidgetryIcon>::new(entity));
            break;
        }
        ancestor = parents.get(entity).ok().map(ChildOf::parent);
    }
}

#[derive(Component, Default)]
struct FieldProjection(Option<Option<RenderedSelection>>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RenderedSelection {
    id: WidgetryListItemId,
    index: usize,
    revision: u64,
}

pub(crate) fn project<T: Send + Sync + 'static>(world: &mut World) {
    let roots = world
        .query::<(
            Entity,
            &WidgetryComboBox<T>,
            &crate::WidgetryComboBoxHeadless<T>,
        )>()
        .iter(world)
        .map(|(root, combo, behavior)| {
            (
                root,
                behavior.source(),
                combo.renderer().clone(),
                combo.placeholder().clone(),
            )
        })
        .collect::<Vec<_>>();
    for (root, source, renderer, placeholder) in roots {
        let result = (|| -> Result<(), RuntimeFailure> {
            let children = part(world.get::<Children>(root), root, ComboPart::Children)?;
            let field = part(
                children
                    .iter()
                    .find(|&field| world.get::<ComboBoxField>(field).is_some()),
                root,
                ComboPart::Field,
            )?;
            let button = part(
                button_content(world, field),
                field,
                ComboPart::ButtonContent,
            )?;
            let content = part(
                world.get::<Children>(button).and_then(|children| {
                    children
                        .iter()
                        .find(|&child| world.get::<ComboBoxFieldContent>(child).is_some())
                }),
                button,
                ComboPart::FieldContent,
            )?;
            let popup = part(
                children
                    .iter()
                    .find(|&popup| world.get::<ComboBoxPopup>(popup).is_some()),
                root,
                ComboPart::Popup,
            )?;
            let list = world
                .get::<Children>(popup)
                .and_then(|children| {
                    children
                        .iter()
                        .find(|&child| world.get::<WidgetryListView<T>>(child).is_some())
                })
                .ok_or(ComboFault::List { source, popup })?;
            let actual_source = world
                .get::<WidgetryListView<T>>(list)
                .map(WidgetryListView::source);
            if actual_source != Some(source) {
                return Err(ComboFault::Mapping {
                    source,
                    list,
                    actual_source,
                }
                .into());
            }
            let selected = part(
                world.get::<WidgetryListSelectionState>(list),
                list,
                ComboPart::Selection,
            )?
            .selected_id();
            let model = world
                .get::<WidgetryListModel<T>>(source)
                .ok_or(ComboFault::Source { source })?;
            let projection = selected
                .map(|id| -> Result<_, ComboFault> {
                    let index = model.index_of(id).ok_or(ComboFault::Item {
                        source,
                        list,
                        item_id: id,
                        index: None,
                        part: ComboPart::Item,
                    })?;
                    Ok(RenderedSelection {
                        id,
                        index,
                        revision: model.revision(index).ok_or(ComboFault::Item {
                            source,
                            list,
                            item_id: id,
                            index: Some(index),
                            part: ComboPart::Revision,
                        })?,
                    })
                })
                .transpose()?;
            if part(
                world.get::<FieldProjection>(content),
                content,
                ComboPart::FieldProjection,
            )?
            .0 == Some(projection)
            {
                return Ok(());
            }
            let frame = || {
                error::frame(root, "combo_box.field_renderer")
                    .field("source", source)
                    .field("list", list)
                    .field("content", content)
                    .field(
                        "item_id",
                        format_args!("{:?}", projection.map(|selected| selected.id)),
                    )
                    .field(
                        "index",
                        format_args!("{:?}", projection.map(|selected| selected.index)),
                    )
                    .field(
                        "revision",
                        format_args!("{:?}", projection.map(|selected| selected.revision)),
                    )
            };
            let scene = projection
                .map(|selected| -> Result<_, RuntimeFailure> {
                    let value = model.get(selected.index).ok_or(ComboFault::Item {
                        source,
                        list,
                        item_id: selected.id,
                        index: Some(selected.index),
                        part: ComboPart::Item,
                    })?;
                    Ok(renderer.render(value).map_err(|failure| {
                        error::list(failure, root, "combo_box.field_renderer").with_context(frame())
                    })?)
                })
                .transpose()?
                .unwrap_or_else(|| placeholder.build());
            let old_children = world
                .get::<Children>(content)
                .map(|children| children.iter().collect::<Vec<_>>())
                .unwrap_or_default();
            world.entity_mut(content).insert(FieldProjection(None));
            for child in old_children {
                world.despawn(child);
            }
            if let Err(failure) =
                apply_scene(&mut world.entity_mut(content), bsn! {Children [{scene}]})
            {
                // A failed expansion must leave the cache invalid and remove only its content subtree.
                let partial = world
                    .get::<Children>(content)
                    .map(|children| children.to_vec())
                    .unwrap_or_default();
                for child in partial {
                    world.despawn(child);
                }
                return Err(RuntimeFailure::External(
                    SceneFailure::from_native(failure, content, "combo_box.field_renderer")
                        .with_context(frame())
                        .into_bevy_error(),
                ));
            }
            world
                .entity_mut(content)
                .insert(FieldProjection(Some(projection)));
            Ok(())
        })();
        let failure = match result {
            Ok(()) => {
                if let Some(mut diagnostics) = world.get_mut::<ComboDiagnostics>(root) {
                    diagnostics
                        .projection
                        .recover(|| error::frame(root, "combo_box.field_project"));
                }
                None
            }
            Err(RuntimeFailure::Fault(fault)) => {
                let failure = match world.get_mut::<ComboDiagnostics>(root) {
                    Some(mut diagnostics) => error::persistent::<T>(
                        &mut diagnostics.projection,
                        fault,
                        root,
                        "combo_box.field_project",
                    ),
                    None => WidgetryEcsError::new(error::failure(
                        fault.into_error::<T>(root),
                        "combo_box.field_project",
                    )),
                };
                Some(failure.into_bevy_error())
            }
            Err(failure) => Some(failure.into_bevy_error::<T>(root, "combo_box.field_project")),
        };
        if let Some(failure) = failure {
            world
                .commands()
                .queue(move |_: &mut World| -> Result<(), BevyError> { Err(failure) });
        }
    }
}

pub(crate) fn scene() -> impl Scene {
    bsn! {
        ComboBoxField
        @WidgetryButton { @children: {Some(Box::new(bsn_list! {
            ComboBoxFieldContent Node { align_items: AlignItems::Center }--
            ComboBoxDropdownIcon
                @WidgetryIcon { @path: {BuiltinIcon::ChevronDown.path()}, @max_size: {Some(UVec2::new(16, 16))} }
                Node { width: px(16), height: px(16), flex_shrink: 0.0 }
        }) as Box<dyn bevy::prelude::SceneList>)}, @content: {Some(Box::new(bsn! { ComboBoxButtonContent Node { flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::SpaceBetween } }) as Box<dyn bevy::prelude::Scene>)}, }
        Node {
            width: percent(100), height: px(36),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(10), px(0)),
            border: UiRect::all(px(1)),
        }

    }
}

pub(crate) fn sync_icon<T: Send + Sync + 'static>(
    popups: Query<(&ChildOf, &Visibility), (With<ComboBoxPopup>, Changed<Visibility>)>,
    mut roots: Query<(&Children, &mut ComboDiagnostics), With<WidgetryComboBox<T>>>,
    fields: Query<&Children, With<ComboBoxField>>,
    layouts: Query<&Children, With<ComboBoxButtonContent>>,
    icons: Query<(), (With<WidgetryIcon>, With<ComboBoxDropdownIcon>)>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let mut batch = DiagnosticBatch::default();
    for (parent, visibility) in &popups {
        let root = parent.parent();
        let Ok((children, mut diagnostics)) = roots.get_mut(root) else {
            continue;
        };
        let icon = children.iter().find_map(|field| {
            fields
                .get(field)
                .ok()?
                .iter()
                .flat_map(|layout| {
                    layouts
                        .get(layout)
                        .into_iter()
                        .flat_map(|children| children.iter())
                })
                .find(|&child| icons.contains(child))
        });
        let Some(icon) = icon else {
            let failure = error::persistent::<T>(
                &mut diagnostics.icon,
                ComboFault::Part {
                    entity: root,
                    part: ComboPart::DropdownIcon,
                },
                root,
                "combo_box.icon",
            );
            for diagnostic in failure.shared_diagnostics() {
                batch.push_shared(diagnostic);
            }
            continue;
        };
        diagnostics
            .icon
            .recover(|| error::frame(root, "combo_box.icon"));
        let path = if *visibility == Visibility::Visible {
            BuiltinIcon::ChevronUp
        } else {
            BuiltinIcon::ChevronDown
        };
        commands.queue(move |world: &mut World| {
            WidgetryIcon::set_svg_in_world(world, icon, path.path())
                .map(|_| ())
                .map_err(|failure| {
                    failure
                        .map_kind(|source| crate::ComboError::Icon {
                            source: Box::new(source),
                        })
                        .with_context(error::frame(root, "combo_box.icon").field("icon", icon))
                        .into_bevy_error()
                })
        });
    }
    batch.into_bevy_error().map_or(Ok(()), Err)
}

pub(crate) fn button_content(world: &World, field: Entity) -> Option<Entity> {
    world
        .get::<Children>(field)?
        .iter()
        .find(|&child| world.get::<ComboBoxButtonContent>(child).is_some())
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{WidgetryComboBoxAppExt, WidgetryComboBoxContent};
    use bevy_widgetry_list_view::WidgetryListViewRenderer;
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};

    #[test]
    fn field_and_icon_state_failures_recover_once_and_recurrence_gets_a_new_origin() {
        for icon_failure in [false, true] {
            let mut app = scene_app();
            app.register_widgetry_combo_box::<u32>().unwrap();
            app.set_error_handler(ErrorCapture::handler());
            app.edit_schedule(PostUpdate, |schedule| {
                schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
            });
            let source = app
                .world_mut()
                .spawn(WidgetryListModel::<u32>::default())
                .id();
            let root=bevy_widgetry_core::scene::spawn_scene(app.world_mut(),bsn!{
                @WidgetryComboBox::<u32> {@source:source,@placeholder:{WidgetryComboBoxContent::new(||bsn_list!{Node})},@renderer:{WidgetryListViewRenderer::new(|_:&u32|bsn_list!{Node})}}
            }).unwrap();
            app.update();
            let field = app.world().get::<Children>(root).unwrap()[0];
            let popup = app.world().get::<Children>(root).unwrap()[1];
            let button = button_content(app.world(), field).unwrap();
            let icon = app
                .world()
                .get::<Children>(button)
                .unwrap()
                .iter()
                .find(|child| app.world().get::<ComboBoxDropdownIcon>(*child).is_some())
                .unwrap();
            let operation = if icon_failure {
                "combo_box.icon"
            } else {
                "combo_box.field_project"
            };
            let errors = ErrorCapture::default();
            let logs = LogCapture::default();
            let mut previous: Option<std::sync::Arc<bevy_widgetry_diagnostics::ErrorOrigin>> = None;
            logs.run(|| {
                for pass in 0..4 {
                    if pass == 2 {
                        if icon_failure {
                            app.world_mut()
                                .entity_mut(icon)
                                .insert(ComboBoxDropdownIcon);
                        } else {
                            app.world_mut().entity_mut(field).insert(ComboBoxField);
                        }
                    } else if icon_failure {
                        app.world_mut()
                            .entity_mut(icon)
                            .remove::<ComboBoxDropdownIcon>();
                    } else {
                        app.world_mut().entity_mut(field).remove::<ComboBoxField>();
                    }
                    app.world_mut().entity_mut(popup).insert(if pass % 2 == 0 {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    });
                    errors.run(||app.update());
                    let failures = errors.take();
                    let diagnostic = failures
                        .iter()
                        .filter_map(|error| error.downcast_ref::<WidgetryEcsError>())
                        .flat_map(|error| error.diagnostics())
                        .find(|diagnostic| {
                            diagnostic
                                .contexts()
                                .iter()
                                .any(|frame| frame.operation() == operation)
                        });
                    if pass == 2 {
                        assert!(diagnostic.is_none());
                        let origin = previous.as_ref().unwrap();
                        assert_eq!(
                            logs.records_for_error(origin.id())
                                .iter()
                                .filter(|record| record.level == bevy::log::Level::INFO)
                                .count(),
                            1
                        );
                        continue;
                    }
                    let diagnostic = diagnostic.unwrap_or_else(||panic!("missing diagnostic, icon={icon_failure}, pass={pass}, failures={failures:?}"));
                    if pass == 1 {
                        assert!(std::sync::Arc::ptr_eq(
                            previous.as_ref().unwrap(),
                            diagnostic.origin()
                        ));
                    }
                    if pass == 3 {
                        assert_ne!(previous.as_ref().unwrap().id(), diagnostic.origin().id());
                    }
                    assert_eq!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                            .len(),
                        1
                    );
                    assert!(
                        logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                            .is_empty()
                    );
                    previous = Some(diagnostic.origin().clone());
                }
            });
        }
    }
}
