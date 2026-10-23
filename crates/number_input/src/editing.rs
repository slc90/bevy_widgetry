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
use bevy_widgetry_core::input_target::WidgetryInputTarget;
use bevy_widgetry_text_field::WidgetryTextField;

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) field: Entity,
}

#[derive(Component, Default)]
struct DisplayFocus(bool);

pub(crate) fn scene<T: WidgetryNumber>(props: WidgetryNumberInputProps<T>) -> impl Scene {
    let text = props
        .value
        .map(|v| format_value(v, props.decimals, false))
        .unwrap_or_default();
    bsn! {
        template(move |_| State::new(&props))
        template(|_|Ok(DisplayFocus::default()))
        Parts {field:#NumberInputField}
        WidgetryInputTarget {target:#NumberInputField}
        Node {width:px(200)}
        Children [
            #NumberInputField @WidgetryTextField {@value:{text.clone()}}
            bevy::input_focus::tab_navigation::TabIndex(0)
            Node {width:percent(100),min_width:px(0)}
        ]
    }
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
    let focused = world.resource::<InputFocus>().get() == Some(editor(world, root)?);
    WidgetryTextField::set_text_in_world(world, field, format_value(value, decimals, focused))?;
    world
        .get_mut::<DisplayFocus>(root)
        .ok_or_else(|| invalid("NumberInput display state is missing"))?
        .0 = focused;
    Ok(())
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
    apply_pending(world, root)?;
    let text = WidgetryTextField::text(world, field(world, root)?)?;
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
    roots: Query<&Parts, With<WidgetryNumberInput<T>>>,
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
        .add_observer(on_focus::<T>);
    app.add_systems(
        PostUpdate,
        prepare_focus::<T>
            .before(bevy::text::EditableTextSystems)
            .before(bevy::ui::UiSystems::Layout),
    );
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

fn ensure_full<T: WidgetryNumber>(world: &mut World, root: Entity) -> Result {
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
    if WidgetryTextField::text(world, field)? == rounded {
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
        WidgetryTextField::set_text_in_world(world, field, value.canonical())?;
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
