use crate::{
    WidgetryNumber, WidgetryNumberInput, WidgetryNumberInputProps,
    value::{State, invalid, state},
};
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{FocusCause, FocusGained, FocusLost, FocusedInput, InputFocus},
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};
use bevy_widgetry_core::{disabled::WidgetryEffectiveDisabled, input_target::WidgetryInputTarget};
use bevy_widgetry_text_field::{WidgetryReadOnlyTextField, WidgetryTextField};

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) field: Entity,
    pub(crate) decrease: Entity,
    pub(crate) increase: Entity,
}

#[derive(Component, FromTemplate)]
pub(crate) struct StepButton {
    pub(crate) root: Entity,
    pub(crate) increase: bool,
}

#[derive(Component, Default)]
struct DisplayFocus(bool);

#[derive(Component, Clone, Default)]
#[component(immutable)]
pub(crate) struct ReadOnly;

#[derive(Component)]
struct PendingDisabled;

pub(crate) fn scene<T: WidgetryNumber>(props: WidgetryNumberInputProps<T>) -> impl Scene {
    let text = props
        .value
        .map(|v| format_value(v, props.decimals, false))
        .unwrap_or_default();
    let read_only = props.read_only.then_some(bsn! { ReadOnly });
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
        template(move |_| State::new(&props))
        template(|_|Ok(DisplayFocus::default()))
        @{read_only}
        Parts {field:#NumberInputField,decrease:#NumberInputDecrease,increase:#NumberInputIncrease}
        WidgetryInputTarget {target:#NumberInputField}
        Node {
            width:px(200),min_width:px(134),min_height:px(38),flex_direction:FlexDirection::Row,
            border:UiRect::all(px(1)),border_radius:BorderRadius::all(px(4)),overflow:Overflow::clip(),
        }
        BorderColor
        BackgroundColor
        Children [
            #NumberInputDecrease StepButton {root:#NumberInputRoot,increase:false}
            @{disabled_button}
            crate::repeating::Repeat
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
            #NumberInputIncrease StepButton {root:#NumberInputRoot,increase:true}
            @{disabled_increase}
            crate::repeating::Repeat
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

pub(crate) fn field(world: &World, root: Entity) -> Result<Entity, BevyError> {
    world
        .get::<Parts>(root)
        .map(|p| p.field)
        .ok_or_else(|| invalid("NumberInput field mapping is missing"))
}

pub(crate) fn editor(world: &World, root: Entity) -> Result<Entity, BevyError> {
    let field = field(world, root)?;
    world
        .get::<WidgetryInputTarget>(field)
        .map(|p| p.target)
        .filter(|e| world.get::<EditableText>(*e).is_some())
        .ok_or_else(|| invalid("NumberInput editor is missing"))
}

pub(crate) fn project<T: WidgetryNumber>(
    world: &mut World,
    root: Entity,
    value: T,
) -> Result<(), BevyError> {
    let field = field(world, root)?;
    let decimals = state::<T>(world, root)?.decimals;
    let focused = !is_disabled(world, root)
        && world.resource::<InputFocus>().get() == Some(editor(world, root)?);
    set_text(world, root, field, format_value(value, decimals, focused))?;
    world
        .get_mut::<DisplayFocus>(root)
        .ok_or_else(|| invalid("NumberInput display state is missing"))?
        .0 = focused;
    Ok(())
}

pub(crate) fn text(world: &World, root: Entity) -> Result<String, BevyError> {
    let field = field(world, root)?;
    if world.get::<ReadOnly>(root).is_some() {
        WidgetryReadOnlyTextField::text(world, field)
    } else {
        WidgetryTextField::text(world, field)
    }
}

fn set_text(world: &mut World, root: Entity, field: Entity, text: String) -> Result {
    if world.get::<ReadOnly>(root).is_some() {
        WidgetryReadOnlyTextField::set_text_in_world(world, field, text)?;
    } else {
        WidgetryTextField::set_text_in_world(world, field, text)?;
    }
    Ok(())
}

pub(crate) fn can_edit(world: &World, root: Entity) -> bool {
    world.get::<ReadOnly>(root).is_none() && !is_disabled(world, root)
}

fn is_disabled(world: &World, root: Entity) -> bool {
    world.get::<bevy::ui::InteractionDisabled>(root).is_some()
        || world
            .get::<WidgetryEffectiveDisabled>(root)
            .is_some_and(|state| state.is_disabled())
}

pub(crate) fn apply_pending(world: &mut World, root: Entity) -> Result<(), BevyError> {
    let editor = editor(world, root)?;
    world.resource_scope(|world, mut fonts: Mut<FontCx>| {
        world.resource_scope(|world, mut layout: Mut<LayoutCx>| {
            world.resource_scope(|world, mut clipboard: Mut<bevy::clipboard::Clipboard>| {
                let mut edit = world
                    .get_mut::<EditableText>(editor)
                    .ok_or_else(|| invalid("NumberInput editor is missing"))?;
                edit.apply_pending_edits(&mut fonts, &mut layout.0, &mut clipboard, |_| true);
                Ok(())
            })
        })
    })
}

fn submit<T: WidgetryNumber>(world: &mut World, root: Entity, blur: bool) -> Result<(), BevyError> {
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
            .ok_or_else(|| invalid("NumberInput display state is missing"))?
            .0
        && text == format_value(state.value, state.decimals, false)
    {
        return Ok(());
    }
    let value = T::parse_decimal(&text).filter(|v| state.accepts(*v));
    if let Some(value) = value {
        WidgetryNumberInput::<T>::set_value_in_world(world, root, value)?;
    } else if blur {
        let value = state.value;
        project(world, root, value)?;
    }
    Ok(())
}

fn on_key<T: WidgetryNumber>(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<
        &Parts,
        (
            With<WidgetryNumberInput<T>>,
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
        if world.get::<WidgetryNumberInput<T>>(root).is_none() {
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
    });
}

fn on_blur<T: WidgetryNumber>(
    event: On<FocusLost>,
    roots: Query<&Parts, With<WidgetryNumberInput<T>>>,
    mut commands: Commands,
) {
    if !roots.contains(event.entity) {
        return;
    }
    let root = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        if world.get::<WidgetryNumberInput<T>>(root).is_none() {
            return Ok(());
        }
        if world.resource::<InputFocus>().get() == Some(editor(world, root)?) {
            return Ok(());
        }
        submit::<T>(world, root, true)
    });
}

pub(crate) fn install<T: WidgetryNumber>(app: &mut App) {
    app.add_observer(on_key::<T>)
        .add_observer(on_blur::<T>)
        .add_observer(on_focus::<T>)
        .add_observer(on_disabled::<T>)
        .add_observer(on_disabled_replaced::<T>);
    app.add_systems(
        PostUpdate,
        prepare_focus::<T>
            .before(bevy::text::EditableTextSystems)
            .before(bevy::ui::UiSystems::Layout),
    );
    app.add_systems(
        PostUpdate,
        restore_pending_disabled::<T>
            .in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Materialize),
    );
}

fn on_disabled<T: WidgetryNumber>(
    event: On<Insert<WidgetryEffectiveDisabled>>,
    roots: Query<&WidgetryEffectiveDisabled, With<WidgetryNumberInput<T>>>,
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
    roots: Query<&WidgetryEffectiveDisabled, With<WidgetryNumberInput<T>>>,
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
        if world.get::<WidgetryNumberInput<T>>(root).is_none() {
            return Ok(());
        }
        // Initial disabled projection can flush before BSN has applied the field's children.
        let field = field(world, root)?;
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
    });
}

fn restore_pending_disabled<T: WidgetryNumber>(
    roots: Query<Entity, (With<WidgetryNumberInput<T>>, With<PendingDisabled>)>,
    mut commands: Commands,
) {
    for root in &roots {
        commands.queue(move |world: &mut World| -> Result {
            if world.get::<WidgetryNumberInput<T>>(root).is_none() {
                return Ok(());
            }
            world.entity_mut(root).remove::<PendingDisabled>();
            editor(world, root)?;
            if !is_disabled(world, root) {
                return Ok(());
            }
            let value = state::<T>(world, root)?.value;
            project(world, root, value)
        });
    }
}

fn prepare_focus<T: WidgetryNumber>(world: &mut World) -> Result {
    let Some(focus) = world.resource::<InputFocus>().get() else {
        return Ok(());
    };
    let root = world
        .query_filtered::<(Entity, &Parts, &DisplayFocus), With<WidgetryNumberInput<T>>>()
        .iter(world)
        .find_map(|(root, parts, display)| {
            (!display.0
                && world
                    .get::<WidgetryInputTarget>(parts.field)
                    .is_some_and(|target| target.target == focus))
            .then_some(root)
        });
    if let Some(root) = root {
        ensure_full::<T>(world, root)?;
    }
    Ok(())
}

pub(crate) fn ensure_full<T: WidgetryNumber>(world: &mut World, root: Entity) -> Result {
    if is_disabled(world, root) {
        return Ok(());
    }
    if world
        .get::<DisplayFocus>(root)
        .ok_or_else(|| invalid("NumberInput display state is missing"))?
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
                .ok_or_else(|| invalid("NumberInput editor is missing"))?;
            (
                std::mem::take(&mut edit.pending_edits),
                edit.pending_paste.take(),
            )
        };
        set_text(world, root, field, value.canonical())?;
        let mut edit = world
            .get_mut::<EditableText>(editor)
            .ok_or_else(|| invalid("NumberInput editor is missing"))?;
        edit.pending_edits.extend(edits);
        edit.pending_paste = paste;
    }
    world
        .get_mut::<DisplayFocus>(root)
        .ok_or_else(|| invalid("NumberInput display state is missing"))?
        .0 = true;
    Ok(())
}

fn on_focus<T: WidgetryNumber>(
    event: On<FocusGained>,
    roots: Query<(), With<WidgetryNumberInput<T>>>,
    mut commands: Commands,
) {
    let root = event.entity;
    if !roots.contains(root) {
        return;
    }
    commands.queue(move |world: &mut World| -> Result {
        if world.get::<WidgetryNumberInput<T>>(root).is_none() {
            return Ok(());
        }
        ensure_full::<T>(world, root)
    });
}
