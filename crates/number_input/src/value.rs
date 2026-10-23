use crate::WidgetryNumber;
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_internal_log::widgetry_error;
use std::marker::PhantomData;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryNumberInputProps<T>)]
pub struct WidgetryNumberInput<T: WidgetryNumber>(PhantomData<fn() -> T>);

#[derive(Clone, Debug)]
pub struct WidgetryNumberInputProps<T: WidgetryNumber> {
    pub value: Option<T>,
    pub min: Option<T>,
    pub max: Option<T>,
    pub step: T,
}

impl<T: WidgetryNumber> Default for WidgetryNumberInputProps<T> {
    fn default() -> Self {
        Self {
            value: None,
            min: None,
            max: None,
            step: T::ONE,
        }
    }
}

#[derive(Component, Reflect)]
#[reflect(Component)]
pub(crate) struct State<T: WidgetryNumber> {
    pub(crate) value: T,
    pub(crate) min: Option<T>,
    pub(crate) max: Option<T>,
    pub(crate) step: T,
}

pub(crate) fn invalid(reason: &str) -> BevyError {
    widgetry_error!(reason, "NumberInput 数值请求无效");
    bevy_widgetry_core::scene::logged_error(reason)
}

impl<T: WidgetryNumber> State<T> {
    fn new(props: &WidgetryNumberInputProps<T>) -> Result<Self, BevyError> {
        let value = props
            .value
            .ok_or_else(|| invalid("NumberInput requires an explicit initial value"))?;
        if props.min.is_some_and(|min| !min.is_finite())
            || props.max.is_some_and(|max| !max.is_finite())
            || matches!((props.min, props.max), (Some(min), Some(max)) if min > max)
        {
            return Err(invalid(
                "NumberInput requires finite bounds with min <= max",
            ));
        }
        if !props.step.is_finite() || props.step <= T::ZERO {
            return Err(invalid("NumberInput step must be finite and positive"));
        }
        let state = Self {
            value,
            min: props.min,
            max: props.max,
            step: props.step,
        };
        state.validate(value)?;
        Ok(state)
    }

    pub(crate) fn validate(&self, value: T) -> Result<(), BevyError> {
        if !value.is_finite()
            || self.min.is_some_and(|min| value < min)
            || self.max.is_some_and(|max| value > max)
        {
            return Err(invalid(
                "NumberInput value must be finite and within its bounds",
            ));
        }
        Ok(())
    }
}

impl<T: WidgetryNumber> WidgetryNumberInput<T> {
    fn scene(props: WidgetryNumberInputProps<T>) -> impl Scene {
        bsn! { template(move |_| State::new(&props)) }
    }

    pub fn value(world: &World, entity: Entity) -> Result<T, BevyError> {
        Ok(state::<T>(world, entity)?.value)
    }

    pub fn set_value_in_world(
        world: &mut World,
        entity: Entity,
        value: T,
    ) -> Result<bool, BevyError> {
        commit(world, entity, value)
    }

    pub fn set_value(commands: &mut Commands, entity: Entity, value: T) {
        commands.queue(move |world: &mut World| {
            Self::set_value_in_world(world, entity, value).map(|_| ())
        });
    }
}

pub(crate) fn commit<T: WidgetryNumber>(
    world: &mut World,
    entity: Entity,
    value: T,
) -> Result<bool, BevyError> {
    let current = state::<T>(world, entity)?;
    current.validate(value)?;
    if current.value == value {
        return Ok(false);
    }
    world
        .get_mut::<State<T>>(entity)
        .ok_or_else(|| invalid("NumberInput runtime state is missing"))?
        .value = value;
    world.trigger(ValueChange {
        source: entity,
        value,
        is_final: true,
    });
    Ok(true)
}

pub(crate) fn state<T: WidgetryNumber>(
    world: &World,
    entity: Entity,
) -> Result<&State<T>, BevyError> {
    if world.get::<WidgetryNumberInput<T>>(entity).is_none() {
        return Err(invalid(
            "NumberInput target does not exist or has the wrong numeric type",
        ));
    }
    world
        .get::<State<T>>(entity)
        .ok_or_else(|| invalid("NumberInput runtime state is missing"))
}
