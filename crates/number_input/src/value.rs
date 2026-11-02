use crate::{
    NumberInputConfigSnapshot, NumberInputError, NumberInputFailure, NumberInputRejection,
    WidgetryNumber,
    error::{failure, missing},
};
use bevy::prelude::*;
use bevy::ui_widgets::ValueChange;
use bevy_widgetry_diagnostics::ErrorFrame;
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
    pub decimals: Option<u8>,
    pub read_only: bool,
}

impl<T: WidgetryNumber> Default for WidgetryNumberInputProps<T> {
    fn default() -> Self {
        Self {
            value: None,
            min: None,
            max: None,
            step: T::ONE,
            decimals: None,
            read_only: false,
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
    pub(crate) decimals: Option<u8>,
}

impl<T: WidgetryNumber> State<T> {
    pub(crate) fn new(props: &WidgetryNumberInputProps<T>) -> Result<Self, NumberInputFailure> {
        let config = || {
            NumberInputConfigSnapshot::new(
                props.value,
                props.min,
                props.max,
                props.step,
                props.decimals,
            )
        };
        let reject = |reason| {
            failure(
                NumberInputError::Numeric {
                    config: Box::new(config()),
                    reason,
                },
                ErrorFrame::new("number_input.initialize_numeric"),
            )
        };
        let value = props
            .value
            .ok_or_else(|| reject(NumberInputRejection::MissingValue))?;
        if props.min.is_some_and(|min| !min.is_finite())
            || props.max.is_some_and(|max| !max.is_finite())
        {
            return Err(reject(NumberInputRejection::NonFiniteBounds));
        }
        if matches!((props.min, props.max), (Some(min), Some(max)) if min > max) {
            return Err(reject(NumberInputRejection::ReversedBounds));
        }
        if !props.step.is_finite() || props.step <= T::ZERO {
            return Err(reject(NumberInputRejection::InvalidStep));
        }
        if props.decimals.is_some() && !T::FLOAT {
            return Err(reject(NumberInputRejection::IntegerDecimals));
        }
        let state = Self {
            value,
            min: props.min,
            max: props.max,
            step: props.step,
            decimals: props.decimals,
        };
        state.validate(value)?;
        Ok(state)
    }

    pub(crate) fn validate(&self, value: T) -> Result<(), NumberInputFailure> {
        if !self.accepts(value) {
            return Err(failure(
                NumberInputError::Numeric {
                    config: Box::new(NumberInputConfigSnapshot::new(
                        Some(value),
                        self.min,
                        self.max,
                        self.step,
                        self.decimals,
                    )),
                    reason: NumberInputRejection::OutOfRange,
                },
                ErrorFrame::new("number_input.validate_value"),
            ));
        }
        Ok(())
    }

    pub(crate) fn accepts(&self, value: T) -> bool {
        value.is_finite()
            && !self.min.is_some_and(|min| value < min)
            && !self.max.is_some_and(|max| value > max)
    }
}

impl<T: WidgetryNumber> WidgetryNumberInput<T> {
    fn scene(props: WidgetryNumberInputProps<T>) -> impl Scene {
        crate::editing::scene(props)
    }

    pub fn value(world: &World, entity: Entity) -> Result<T, NumberInputFailure> {
        Ok(state::<T>(world, entity)?.value)
    }

    pub fn set_value_in_world(
        world: &mut World,
        entity: Entity,
        value: T,
    ) -> Result<bool, NumberInputFailure> {
        commit(world, entity, value)
    }

    pub fn set_value(commands: &mut Commands, entity: Entity, value: T) {
        commands.queue(move |world: &mut World| {
            Self::set_value_in_world(world, entity, value)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }
}

pub(crate) fn commit<T: WidgetryNumber>(
    world: &mut World,
    entity: Entity,
    value: T,
) -> Result<bool, NumberInputFailure> {
    let current = state::<T>(world, entity)?;
    current.validate(value).map_err(|error| {
        error.with_context(ErrorFrame::new("number_input.set_value").field("root", entity))
    })?;
    let changed = current.value != value;
    crate::editing::project(world, entity, value)?;
    if !changed {
        return Ok(false);
    }
    world
        .get_mut::<State<T>>(entity)
        .ok_or_else(|| missing(entity, entity, "State", "number_input.state"))?
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
) -> Result<&State<T>, NumberInputFailure> {
    if world.get::<WidgetryNumberInput<T>>(entity).is_none() {
        return Err(failure(
            NumberInputError::InvalidRoot {
                root: entity,
                numeric_type: std::any::type_name::<T>(),
            },
            ErrorFrame::new("number_input.state"),
        ));
    }
    world
        .get::<State<T>>(entity)
        .ok_or_else(|| missing(entity, entity, "State", "number_input.state"))
}
