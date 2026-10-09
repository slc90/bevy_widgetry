use bevy::prelude::*;
use bevy_widgetry_log::widgetry_error;
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

pub fn validate_color(color: Color) -> Result<(), BevyError> {
    let rgba = color.to_linear();
    if [rgba.red, rgba.green, rgba.blue, rgba.alpha]
        .iter()
        .all(|v| v.is_finite())
        && (0.0..=1.0).contains(&rgba.alpha)
    {
        return Ok(());
    }
    widgetry_error!(?color, "颜色分量必须有限且 alpha 位于 [0,1]");
    Err(BevyError::error("颜色分量必须有限且 alpha 位于 [0,1]"))
}
