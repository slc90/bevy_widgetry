use crate::{
    WidgetryNumber, WidgetryNumberInput, WidgetryNumberInputProps,
    value::{State, invalid, state},
};
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::{FocusCause, FocusLost, FocusedInput, InputFocus},
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};
use bevy_widgetry_core::input_target::WidgetryInputTarget;
use bevy_widgetry_text_field::WidgetryTextField;

#[derive(Component, FromTemplate)]
pub(crate) struct Parts {
    pub(crate) field: Entity,
}

pub(crate) fn scene<T: WidgetryNumber>(props: WidgetryNumberInputProps<T>) -> impl Scene {
    let text = props.value.map(T::canonical).unwrap_or_default();
    bsn! {
        template(move |_| State::new(&props))
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
    WidgetryTextField::set_text_in_world(world, field, value.canonical()).map(|_| ())
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
    app.add_observer(on_key::<T>).add_observer(on_blur::<T>);
}
