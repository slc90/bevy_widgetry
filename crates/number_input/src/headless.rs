use crate::{NumberInputFailure, WidgetryNumber, WidgetryNumberInputProps};
use bevy::{
    ecs::{lifecycle::HookContext, world::DeferredWorld},
    prelude::*,
};
use bevy_widgetry_core::input_target::WidgetryInputTarget;
use std::marker::PhantomData;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryNumberInputProps<T>)]
#[require(Node)]
pub struct WidgetryNumberInputHeadless<T: WidgetryNumber>(PhantomData<fn() -> T>);

impl<T: WidgetryNumber> WidgetryNumberInputHeadless<T> {
    fn scene(props: WidgetryNumberInputProps<T>) -> impl Scene {
        crate::editing::behavior(props)
    }
}

#[derive(Component, FromTemplate)]
#[component(immutable, on_insert = bind_input_target)]
#[require(WidgetryInputTarget = WidgetryInputTarget {target:Entity::PLACEHOLDER})]
pub struct WidgetryNumberInputEditor {
    pub field: Entity,
}

fn bind_input_target(mut world: DeferredWorld, context: HookContext) {
    if let Some(field) = world
        .get::<WidgetryNumberInputEditor>(context.entity)
        .map(|binding| binding.field)
        && let Some(mut target) = world.get_mut::<WidgetryInputTarget>(context.entity)
    {
        target.target = field;
    }
}

impl<T: WidgetryNumber> WidgetryNumberInputHeadless<T> {
    pub fn value(world: &World, root: Entity) -> Result<T, NumberInputFailure> {
        Ok(crate::value::state::<T>(world, root)?.value)
    }
    pub fn set_value_in_world(
        world: &mut World,
        root: Entity,
        value: T,
    ) -> Result<bool, NumberInputFailure> {
        crate::value::commit(world, root, value)
    }
    pub fn set_value(commands: &mut Commands, root: Entity, value: T) {
        commands.queue(move |world: &mut World| {
            Self::set_value_in_world(world, root, value)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
    pub fn field_entity(world: &World, root: Entity) -> Result<Entity, NumberInputFailure> {
        crate::value::state::<T>(world, root)?;
        crate::editing::editor(world, root)?;
        crate::editing::field(world, root)
    }
    pub fn editor_entity(world: &World, root: Entity) -> Result<Entity, NumberInputFailure> {
        crate::value::state::<T>(world, root)?;
        crate::editing::editor(world, root)
    }
}
