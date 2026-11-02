use crate::{
    NumberInputFailure,
    editing::Parts,
    error::{missing, text_field},
};
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
            let result = (|| -> Result<(), NumberInputFailure> {
                if world.get::<Parts>(root).is_none() {
                    return Ok(());
                }
                for button in [decrease, increase] {
                    if world.get::<Node>(button).is_none() {
                        return Err(missing(
                            root,
                            button,
                            "Node",
                            "number_input.initialize_layout",
                        ));
                    }
                }
                let frame =
                    bevy_widgetry_text_field::internal::frame(world, field).map_err(|error| {
                        text_field(error, root, field, "number_input.initialize_layout")
                    })?;
                let mut node = world.get_mut::<Node>(frame).ok_or_else(|| {
                    missing(root, frame, "Node", "number_input.initialize_layout")
                })?;
                node.border = UiRect::ZERO;
                node.border_radius = BorderRadius::ZERO;
                Ok(())
            })();
            result.map_err(|error| error.into_bevy_error())
        });
    }
}
