use crate::diagnostics::{CoreError, CoreFailure, failure};
use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(Component)]
pub struct WidgetryStyleOwner<T: Send + Sync + 'static> {
    pub entity: Entity,
    marker: PhantomData<fn() -> T>,
}

impl<T: Send + Sync + 'static> WidgetryStyleOwner<T> {
    pub fn new(entity: Entity) -> Self {
        Self {
            entity,
            marker: PhantomData,
        }
    }
}

pub fn validate_color(color: Color) -> Result<(), CoreFailure> {
    let rgba = color.to_linear();
    if [rgba.red, rgba.green, rgba.blue, rgba.alpha]
        .iter()
        .all(|v| v.is_finite())
        && (0.0..=1.0).contains(&rgba.alpha)
    {
        return Ok(());
    }
    Err(failure(CoreError::InvalidColor { color }, "validate color"))
}
