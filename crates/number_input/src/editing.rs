use crate::{
    NumberInputError, NumberInputFailure,
    error::{failure, missing, text_field},
};
use crate::{
    WidgetryNumber, WidgetryNumberInputEditor, WidgetryNumberInputHeadless,
    WidgetryNumberInputProps, WidgetryNumberInputStep,
    value::{State, state},
};
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{FocusCause, FocusGained, FocusLost, FocusedInput, InputFocus},
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};
use bevy_widgetry_core::{disabled::WidgetryEffectiveDisabled, input_target::WidgetryInputTarget};
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_text_field::WidgetryTextFieldHeadless;
use bevy_widgetry_text_field::{WidgetryReadOnlyTextField, WidgetryTextField};

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) field: Entity,
    pub(crate) decrease: Entity,
    pub(crate) increase: Entity,
}

#[derive(Component, Default)]
struct DisplayFocus(bool);

#[derive(Component, Clone, Default)]
#[component(immutable)]
pub(crate) struct ReadOnly;

#[derive(Component)]
struct PendingDisabled;

#[derive(Component)]
struct PendingInitialization;

pub(crate) fn behavior<T: WidgetryNumber>(props: WidgetryNumberInputProps<T>) -> impl Scene {
    let read_only = props.read_only.then_some(bsn! { ReadOnly });
    bsn! {
        template(move |context| State::new(&props).map_err(|error| error.with_context(ErrorFrame::new("number_input.initialize").field("root", context.entity.id())).into_bevy_error()))
        template(|_|Ok(DisplayFocus::default()))
        template(|_|Ok(PendingInitialization))
        template(|_|Ok(crate::input::StepTarget(crate::input::step::<T>)))
        @{read_only}
    }
}

pub(crate) fn scene<T: WidgetryNumber>(props: WidgetryNumberInputProps<T>) -> impl Scene {
    let text = props
        .value
        .map(|v| format_value(v, props.decimals, false))
        .unwrap_or_default();
    let field_scene: Box<dyn Scene> = if props.read_only {
        Box::new(bsn! { @WidgetryReadOnlyTextField { @value: {text.clone()} } })
    } else {
        Box::new(bsn! { @WidgetryTextField { @value: {text.clone()} } })
    };
    let disabled_button = props
        .read_only
        .then_some(bsn! { bevy::ui::InteractionDisabled });
    let disabled_increase = props
        .read_only
        .then_some(bsn! { bevy::ui::InteractionDisabled });
    bsn! {
        @root_reference(#NumberInputRoot)
        @WidgetryNumberInputHeadless<T> {@value:{props.value},@min:{props.min},@max:{props.max},@step:{props.step},@decimals:{props.decimals},@read_only:{props.read_only}}
        WidgetryNumberInputEditor {field:#NumberInputField}
        Parts {field:#NumberInputField,decrease:#NumberInputDecrease,increase:#NumberInputIncrease}
        Node {
            width:px(200),min_width:px(134),min_height:px(38),flex_direction:FlexDirection::Row,
            border:UiRect::all(px(1)),border_radius:BorderRadius::all(px(4)),overflow:Overflow::clip(),
        }
        BorderColor
        BackgroundColor
        Children [
            #NumberInputDecrease WidgetryNumberInputStep {root:#NumberInputRoot,increase:false}
            @{disabled_button}
            crate::WidgetryNumberInputRepeat
            bevy::ui_widgets::Button bevy::ui_widgets::ActivateOnPress
            bevy_widgetry_core::pointer::WidgetryPointerPressed bevy::picking::hover::Hovered(false)
            Node {width:px(36),min_height:px(36),flex_shrink:0.0,align_items:AlignItems::Center,justify_content:JustifyContent::Center,border:UiRect::right(px(1)),border_radius:BorderRadius::left(px(3))}
            BorderColor
            BackgroundColor bevy_widgetry_core::foreground::ResolvedForeground
            Children [Text::new("−") bevy_widgetry_core::text::WidgetryText Pickable::IGNORE]--
            #NumberInputField @{field_scene}
            bevy::input_focus::tab_navigation::TabIndex(0)
            TextLayout {justify:Justify::Center}
            Node {flex_grow:1.0,flex_basis:px(0),min_width:px(60)}--
            #NumberInputIncrease WidgetryNumberInputStep {root:#NumberInputRoot,increase:true}
            @{disabled_increase}
            crate::WidgetryNumberInputRepeat
            bevy::ui_widgets::Button bevy::ui_widgets::ActivateOnPress
            bevy_widgetry_core::pointer::WidgetryPointerPressed bevy::picking::hover::Hovered(false)
            Node {width:px(36),min_height:px(36),flex_shrink:0.0,align_items:AlignItems::Center,justify_content:JustifyContent::Center,border:UiRect::left(px(1)),border_radius:BorderRadius::right(px(3))}
            BorderColor
            BackgroundColor bevy_widgetry_core::foreground::ResolvedForeground
            Children [Text::new("+") bevy_widgetry_core::text::WidgetryText Pickable::IGNORE]
        ]
    }
}

fn root_reference(reference: bevy::ecs::template::EntityTemplate) -> impl Scene {
    bevy::scene::SceneFunction(move |_, scene| {
        if let bevy::ecs::template::EntityTemplate::SceneEntityReference(reference) = reference {
            scene.entity_references.push(reference);
        }
    })
}

fn format_value<T: WidgetryNumber>(value: T, decimals: Option<u8>, focused: bool) -> String {
    if !focused && let Some(decimals) = decimals {
        format!("{value:.precision$}", precision = usize::from(decimals))
    } else {
        value.canonical()
    }
}

pub(crate) fn field(world: &World, root: Entity) -> Result<Entity, NumberInputFailure> {
    world
        .get::<WidgetryNumberInputEditor>(root)
        .map(|p| p.field)
        .ok_or_else(|| {
            missing(
                root,
                root,
                "WidgetryNumberInputEditor",
                "number_input.field",
            )
        })
}

pub(crate) fn editor(world: &World, root: Entity) -> Result<Entity, NumberInputFailure> {
    let field = field(world, root)?;
    let read_only = world.get::<ReadOnly>(root).is_some();
    let mut ancestor = field;
    while ancestor != root {
        let Some(parent) = world.get::<ChildOf>(ancestor) else {
            return Err(failure(
                NumberInputError::InvalidEditorBinding {
                    root,
                    field,
                    read_only,
                },
                ErrorFrame::new("number_input.editor"),
            ));
        };
        ancestor = parent.parent();
    }
    if field == root
        || WidgetryTextFieldHeadless::is_read_only(world, field)
            .map_err(|error| text_field(error, root, field, "number_input.editor"))?
            != read_only
    {
        return Err(failure(
            NumberInputError::InvalidEditorBinding {
                root,
                field,
                read_only,
            },
            ErrorFrame::new("number_input.editor"),
        ));
    }
    let target = world
        .get::<WidgetryInputTarget>(field)
        .ok_or_else(|| missing(root, field, "WidgetryInputTarget", "number_input.editor"))?
        .target;
    let bound = WidgetryTextFieldHeadless::editor_entity(world, field)
        .map_err(|error| text_field(error, root, field, "number_input.editor"))?;
    if world.get::<EditableText>(target).is_none() {
        return Err(
            missing(root, target, "EditableText", "number_input.editor").with_context(
                ErrorFrame::new("number_input.validate_input_target").field("field", field),
            ),
        );
    }
    if target != bound {
        return Err(text_field(
            bevy_widgetry_text_field::TextFieldFailure::new(
                bevy_widgetry_text_field::TextFieldError::InvalidEditorBinding {
                    root: field,
                    editor: target,
                    bound,
                },
                ErrorFrame::new("number_input.validate_input_target"),
            ),
            root,
            field,
            "number_input.editor",
        ));
    }
    let max_length = WidgetryTextFieldHeadless::max_length(world, field)
        .map_err(|error| text_field(error, root, field, "number_input.editor"))?;
    let multiline = WidgetryTextFieldHeadless::editor(world, field)
        .map_err(|error| text_field(error, root, field, "number_input.editor"))?
        .allow_newlines;
    if max_length.is_some() || multiline {
        return Err(failure(
            NumberInputError::IncompatibleEditor {
                root,
                field,
                editor: bound,
                max_length,
                multiline,
            },
            ErrorFrame::new("number_input.editor"),
        ));
    }
    Ok(bound)
}

pub(crate) fn project<T: WidgetryNumber>(
    world: &mut World,
    root: Entity,
    value: T,
) -> Result<(), NumberInputFailure> {
    let field = field(world, root)?;
    let decimals = state::<T>(world, root)?.decimals;
    let focus = world
        .get_resource::<InputFocus>()
        .ok_or_else(|| {
            failure(
                NumberInputError::MissingResource {
                    root,
                    resource: "InputFocus",
                },
                ErrorFrame::new("number_input.project"),
            )
        })?
        .get();
    let editor = editor(world, root)?;
    let focused = !is_disabled(world, root) && focus == Some(editor);
    set_text(world, root, field, format_value(value, decimals, focused))?;
    world
        .get_mut::<DisplayFocus>(root)
        .ok_or_else(|| {
            missing(
                root,
                root,
                "DisplayFocus",
                "number_input.project.after_text",
            )
            .with_context(
                ErrorFrame::new("number_input.project")
                    .field("field", field)
                    .field("text_written", true),
            )
        })?
        .0 = focused;
    world.entity_mut(root).remove::<PendingInitialization>();
    Ok(())
}

pub(crate) fn text(world: &World, root: Entity) -> Result<String, NumberInputFailure> {
    let field = field(world, root)?;
    let result = WidgetryTextFieldHeadless::text(world, field);
    result.map_err(|error| text_field(error, root, field, "number_input.read_text"))
}
fn set_text(
    world: &mut World,
    root: Entity,
    field: Entity,
    text: String,
) -> Result<(), NumberInputFailure> {
    let result = WidgetryTextFieldHeadless::set_text_in_world(world, field, text);
    result.map_err(|error| text_field(error, root, field, "number_input.set_text"))?;
    Ok(())
}

pub(crate) fn can_edit(world: &World, root: Entity) -> bool {
    world.get::<ReadOnly>(root).is_none() && !is_disabled(world, root)
}

pub(crate) fn is_disabled(world: &World, root: Entity) -> bool {
    world.get::<bevy::ui::InteractionDisabled>(root).is_some()
        || world
            .get::<WidgetryEffectiveDisabled>(root)
            .is_some_and(|state| state.is_disabled())
}

pub(crate) fn apply_pending(world: &mut World, root: Entity) -> Result<(), NumberInputFailure> {
    let editor = editor(world, root)?;
    for (present, resource) in [
        (world.contains_resource::<FontCx>(), "FontCx"),
        (world.contains_resource::<LayoutCx>(), "LayoutCx"),
        (
            world.contains_resource::<bevy::clipboard::Clipboard>(),
            "Clipboard",
        ),
    ] {
        if !present {
            return Err(failure(
                NumberInputError::MissingResource { root, resource },
                ErrorFrame::new("number_input.apply_pending").field("editor", editor),
            ));
        }
    }
    world.resource_scope(|world, mut fonts: Mut<FontCx>| {
        world.resource_scope(|world, mut layout: Mut<LayoutCx>| {
            world.resource_scope(|world, mut clipboard: Mut<bevy::clipboard::Clipboard>| {
                let mut edit = world
                    .get_mut::<EditableText>(editor)
                    .ok_or_else(|| missing(root, editor, "EditableText", "number_input.editing"))?;
                edit.apply_pending_edits(&mut fonts, &mut layout.0, &mut clipboard, |_| true);
                Ok(())
            })
        })
    })
}

fn submit<T: WidgetryNumber>(
    world: &mut World,
    root: Entity,
    blur: bool,
) -> Result<(), NumberInputFailure> {
    if !can_edit(world, root) {
        if blur {
            let value = state::<T>(world, root)?.value;
            project(world, root, value)?;
        }
        return Ok(());
    }
    apply_pending(world, root)?;
    let text = text(world, root)?;
    let state = state::<T>(world, root)?;
    // A setter can project rounded text after focus moved but before its queued FocusLost arrives.
    // That projection is already synchronized and must never become a numeric submission.
    if blur
        && !world
            .get::<DisplayFocus>(root)
            .ok_or_else(|| missing(root, root, "DisplayFocus", "number_input.display_state"))?
            .0
        && text == format_value(state.value, state.decimals, false)
    {
        return Ok(());
    }
    let value = T::parse_decimal(&text).filter(|v| state.accepts(*v));
    if let Some(value) = value {
        WidgetryNumberInputHeadless::<T>::set_value_in_world(world, root, value)?;
    } else if blur {
        let value = state.value;
        project(world, root, value)?;
    }
    Ok(())
}

fn on_key<T: WidgetryNumber>(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<
        &WidgetryNumberInputEditor,
        (
            With<WidgetryNumberInputHeadless<T>>,
            Without<ReadOnly>,
            Without<bevy::ui::InteractionDisabled>,
        ),
    >,
    targets: Query<&WidgetryInputTarget>,
    mut commands: Commands,
) {
    let root = event.focused_entity;
    let Ok(parts) = roots.get(root) else { return };
    if !event.input.state.is_pressed()
        || !matches!(
            event.input.key_code,
            KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape
        )
        || !targets
            .get(parts.field)
            .is_ok_and(|target| target.target == event.original_event_target())
    {
        return;
    }
    let cancel = event.input.key_code == KeyCode::Escape;
    event.propagate(false);
    commands.queue(move |world: &mut World| -> Result {
        let result = (|| -> Result<(), NumberInputFailure> {
            if world.get::<WidgetryNumberInputHeadless<T>>(root).is_none() {
                return Ok(());
            }
            if !can_edit(world, root) {
                return Ok(());
            }
            ensure_full::<T>(world, root)?;
            if cancel {
                let value = state::<T>(world, root)?.value;
                project(world, root, value)?;
                let editor = editor(world, root)?;
                world
                    .resource_mut::<InputFocus>()
                    .set(editor, FocusCause::Navigated);
                Ok(())
            } else {
                submit::<T>(world, root, false)
            }
        })();
        result.map_err(|error| error.into_bevy_error())
    });
}

fn on_blur<T: WidgetryNumber>(
    event: On<FocusLost>,
    roots: Query<&WidgetryNumberInputEditor, With<WidgetryNumberInputHeadless<T>>>,
    mut commands: Commands,
) {
    if !roots.contains(event.entity) {
        return;
    }
    let root = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        let result = (|| -> Result<(), NumberInputFailure> {
            if world.get::<WidgetryNumberInputHeadless<T>>(root).is_none() {
                return Ok(());
            }
            if let Some(mut focused) = world.resource::<InputFocus>().get() {
                loop {
                    if focused == root {
                        return Ok(());
                    }
                    let Some(parent) = world.get::<ChildOf>(focused) else {
                        break;
                    };
                    focused = parent.parent();
                }
            }
            submit::<T>(world, root, true)
        })();
        result.map_err(|error| error.into_bevy_error())
    });
}

pub(crate) fn install<T: WidgetryNumber>(app: &mut App) {
    app.add_observer(on_key::<T>)
        .add_observer(on_blur::<T>)
        .add_observer(on_focus::<T>)
        .add_observer(on_disabled::<T>)
        .add_observer(on_disabled_replaced::<T>);
    app.add_systems(
        PreUpdate,
        initialize::<T>.before(bevy::input_focus::InputFocusSystems::Dispatch),
    );
    app.add_systems(
        PostUpdate,
        prepare_focus::<T>
            .before(bevy::text::EditableTextSystems)
            .before(bevy::ui::UiSystems::Layout),
    );
    app.add_systems(
        PostUpdate,
        (initialize::<T>, restore_pending_disabled::<T>)
            .chain()
            .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Materialize),
    );
}

fn initialize<T: WidgetryNumber>(
    roots: Query<
        Entity,
        (
            With<WidgetryNumberInputHeadless<T>>,
            With<PendingInitialization>,
        ),
    >,
    mut commands: Commands,
) {
    for root in &roots {
        commands.queue(move |world: &mut World| -> Result {
            if world.get::<WidgetryNumberInputHeadless<T>>(root).is_none() {
                return Ok(());
            }
            if world.get::<PendingInitialization>(root).is_none() {
                return Ok(());
            }
            world.entity_mut(root).remove::<PendingInitialization>();
            if world.get::<crate::WidgetryNumberInput<T>>(root).is_some() {
                editor(world, root).map_err(|error| error.into_bevy_error())?;
                return Ok(());
            }
            let value = state::<T>(world, root)
                .map_err(|error| error.into_bevy_error())?
                .value;
            project(world, root, value).map_err(|error| error.into_bevy_error())
        });
    }
}

fn on_disabled<T: WidgetryNumber>(
    event: On<Insert<WidgetryEffectiveDisabled>>,
    roots: Query<&WidgetryEffectiveDisabled, With<WidgetryNumberInputHeadless<T>>>,
    mut commands: Commands,
) {
    if !roots
        .get(event.entity)
        .is_ok_and(|disabled| disabled.is_disabled())
    {
        return;
    }
    queue_disabled_restore::<T>(event.entity, &mut commands);
}

fn on_disabled_replaced<T: WidgetryNumber>(
    event: On<Discard<WidgetryEffectiveDisabled>>,
    roots: Query<&WidgetryEffectiveDisabled, With<WidgetryNumberInputHeadless<T>>>,
    mut commands: Commands,
) {
    // Discard observes the old state, including a disabled-to-enabled replacement.
    if roots
        .get(event.entity)
        .is_ok_and(|state| state.is_disabled())
    {
        queue_disabled_restore::<T>(event.entity, &mut commands);
    }
}

fn queue_disabled_restore<T: WidgetryNumber>(root: Entity, commands: &mut Commands) {
    commands.queue(move |world: &mut World| -> Result {
        let result = (|| -> Result<(), NumberInputFailure> {
            if world.get::<WidgetryNumberInputHeadless<T>>(root).is_none() {
                return Ok(());
            }
            // Initial disabled projection can flush before BSN has applied the field's children.
            let Some(field) = world
                .get::<WidgetryNumberInputEditor>(root)
                .map(|binding| binding.field)
            else {
                world.entity_mut(root).insert(PendingDisabled);
                return Ok(());
            };
            if world
                .get::<WidgetryInputTarget>(field)
                .and_then(|target| world.get::<EditableText>(target.target))
                .is_none()
            {
                world.entity_mut(root).insert(PendingDisabled);
                return Ok(());
            }
            let value = state::<T>(world, root)?.value;
            project(world, root, value)
        })();
        result.map_err(|error| error.into_bevy_error())
    });
}

fn restore_pending_disabled<T: WidgetryNumber>(
    roots: Query<Entity, (With<WidgetryNumberInputHeadless<T>>, With<PendingDisabled>)>,
    mut commands: Commands,
) {
    for root in &roots {
        commands.queue(move |world: &mut World| -> Result {
            let result = (|| -> Result<(), NumberInputFailure> {
                if world.get::<WidgetryNumberInputHeadless<T>>(root).is_none() {
                    return Ok(());
                }
                world.entity_mut(root).remove::<PendingDisabled>();
                editor(world, root)?;
                if !is_disabled(world, root) {
                    return Ok(());
                }
                let value = state::<T>(world, root)?.value;
                project(world, root, value)
            })();
            result.map_err(|error| error.into_bevy_error())
        });
    }
}

fn prepare_focus<T: WidgetryNumber>(world: &mut World) -> Result {
    let Some(focus) = world.resource::<InputFocus>().get() else {
        return Ok(());
    };
    let mut ancestor = focus;
    loop {
        if world
            .get::<WidgetryNumberInputHeadless<T>>(ancestor)
            .is_some()
        {
            if editor(world, ancestor).map_err(|error| error.into_bevy_error())? == focus {
                ensure_full::<T>(world, ancestor).map_err(|error| error.into_bevy_error())?;
            }
            break;
        }
        let Some(parent) = world.get::<ChildOf>(ancestor) else {
            break;
        };
        ancestor = parent.parent();
    }
    Ok(())
}

pub(crate) fn ensure_full<T: WidgetryNumber>(
    world: &mut World,
    root: Entity,
) -> Result<(), NumberInputFailure> {
    if is_disabled(world, root) {
        return Ok(());
    }
    if world
        .get::<DisplayFocus>(root)
        .ok_or_else(|| missing(root, root, "DisplayFocus", "number_input.display_state"))?
        .0
    {
        return Ok(());
    }
    let state = state::<T>(world, root)?;
    if state.decimals.is_none() {
        return Ok(());
    }
    let value = state.value;
    let rounded = format_value(value, state.decimals, false);
    let field = field(world, root)?;
    if text(world, root)? == rounded {
        let editor = editor(world, root)?;
        let (edits, paste) = {
            let mut edit = world
                .get_mut::<EditableText>(editor)
                .ok_or_else(|| missing(root, editor, "EditableText", "number_input.editing"))?;
            (
                std::mem::take(&mut edit.pending_edits),
                edit.pending_paste.take(),
            )
        };
        set_text(world, root, field, value.canonical())?;
        let mut edit = world
            .get_mut::<EditableText>(editor)
            .ok_or_else(|| missing(root, editor, "EditableText", "number_input.editing"))?;
        edit.pending_edits.extend(edits);
        edit.pending_paste = paste;
    }
    world
        .get_mut::<DisplayFocus>(root)
        .ok_or_else(|| missing(root, root, "DisplayFocus", "number_input.display_state"))?
        .0 = true;
    Ok(())
}

fn on_focus<T: WidgetryNumber>(
    event: On<FocusGained>,
    roots: Query<(), With<WidgetryNumberInputHeadless<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if !roots.contains(root) {
        return;
    }
    commands.queue(move |world: &mut World| -> Result {
        let result = (|| -> Result<(), NumberInputFailure> {
            if world.get::<WidgetryNumberInputHeadless<T>>(root).is_none() {
                return Ok(());
            }
            ensure_full::<T>(world, root)
        })();
        result.map_err(|error| error.into_bevy_error())
    });
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod diagnostic_tests {
    use super::*;
    use crate::WidgetryNumberInput;
    use bevy_widgetry_core::scene::spawn_scene;
    use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};

    #[test]
    fn text_field_failure_keeps_origin_and_number_input_context() {
        let mut app = bevy_widgetry_test_utils::scene_app();
        app.add_plugins(crate::WidgetryNumberInputPlugin);
        app.set_error_handler(ErrorCapture::handler());
        let root = spawn_scene(
            app.world_mut(),
            bsn! { @WidgetryNumberInput::<f64> { @value: {Some(5.0)} } },
        )
        .unwrap();
        let editor = editor(app.world(), root).unwrap();
        app.world_mut().entity_mut(editor).remove::<EditableText>();
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| {
            errors.run(|| {
                app.world_mut().commands().queue(move |world: &mut World| {
                    text(world, root)
                        .map(|_| ())
                        .map_err(|error| error.into_bevy_error())
                });
                app.world_mut().flush();
            })
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(
            diagnostic
                .contexts()
                .iter()
                .any(|frame| frame.operation() == "number_input.read_text")
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert_eq!(logs.records().len(), 1);
    }
}
