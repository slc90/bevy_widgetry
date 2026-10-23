use crate::{editing::Parts, value::invalid};
use bevy::prelude::*;
use bevy_widgetry_core::ui::WidgetryUiSystems;

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        initialize.in_set(WidgetryUiSystems::Materialize),
    );
}

fn initialize(roots: Query<(Entity, &Parts), Added<Parts>>, mut commands: Commands) {
    for (root, parts) in &roots {
        let (field, decrease, increase) = (parts.field, parts.decrease, parts.increase);
        commands.queue(move |world: &mut World| -> Result {
            if world.get::<Parts>(root).is_none() {
                return Ok(());
            }
            if world.get::<Node>(decrease).is_none() || world.get::<Node>(increase).is_none() {
                return Err(invalid("NumberInput step button is missing"));
            }
            let frame = bevy_widgetry_text_field::internal::frame(world, field)?;
            let mut node = world
                .get_mut::<Node>(frame)
                .ok_or_else(|| invalid("NumberInput text frame is missing"))?;
            node.border = UiRect::ZERO;
            node.border_radius = BorderRadius::ZERO;
            Ok(())
        });
    }
}
