use bevy::{
    camera::NormalizedRenderTarget,
    ecs::{
        entity::{ContainsEntity, EntityHashMap},
        message::MessageCursor,
    },
    picking::{
        PickingSystems,
        backend::PointerHits,
        events::pointer_events,
        hover::{HoverMap, PointerCaptureMap, generate_hovermap},
        pointer::{PointerId, PointerLocation},
    },
    prelude::*,
};

#[derive(Resource, Default)]
struct HitFrame {
    ids: Vec<usize>,
    events_started: bool,
}

pub(super) fn install(app: &mut App) {
    if app.world().contains_resource::<HitFrame>() {
        return;
    }
    app.init_resource::<HitFrame>()
        .add_systems(
            PreUpdate,
            capture_hits
                .in_set(PickingSystems::Hover)
                .before(generate_hovermap),
        )
        .add_systems(
            PreUpdate,
            mark_dispatch
                .in_set(PickingSystems::Hover)
                .after(capture_hits)
                .after(super::update_hover)
                .before(pointer_events),
        );
}

fn capture_hits(
    hits: Option<Res<Messages<PointerHits>>>,
    mut cursor: Local<MessageCursor<PointerHits>>,
    mut frame: ResMut<HitFrame>,
) {
    frame.events_started = false;
    frame.ids.clear();
    if let Some(hits) = hits {
        frame
            .ids
            .extend(cursor.read_with_id(&hits).map(|(_, id)| id.id));
    }
}

fn mark_dispatch(mut frame: ResMut<HitFrame>) {
    frame.events_started = true;
}

pub fn refresh_hover(world: &mut World, pointer: PointerId) {
    // Once the event batch has started, changing HoverMap would erase the next frame's exit/entry transition.
    // Leave that transition for generate_hovermap instead of silently advancing its previous state.
    if world
        .get_resource::<HitFrame>()
        .is_some_and(|frame| frame.events_started)
    {
        return;
    }
    if !world.contains_resource::<HoverMap>() {
        return;
    }
    let valid = world
        .query::<(&PointerId, &PointerLocation)>()
        .iter(world)
        .any(|(id, location)| {
            *id == pointer
                && location.location.as_ref().is_some_and(|location| {
                    location.position.is_finite()
                        && match location.target {
                            NormalizedRenderTarget::Window(window) => {
                                world.get::<Window>(window.entity()).is_some()
                            }
                            _ => true,
                        }
                })
        });
    let mut resolved = EntityHashMap::default();
    if valid {
        let captured = world
            .get_resource::<PointerCaptureMap>()
            .and_then(|capture| capture.get(&pointer))
            .filter(|(entity, _)| world.entities().contains(*entity))
            .map(|(entity, hit)| (entity, hit.clone()));
        if let Some((entity, hit)) = captured {
            resolved.insert(entity, hit);
        } else {
            // Cache message IDs at the backend boundary so restoration cannot replay an older frame's hits.
            let mut ordered = world
                .get_resource::<HitFrame>()
                .into_iter()
                .flat_map(|frame| &frame.ids)
                .filter_map(|id| {
                    world
                        .get_resource::<Messages<PointerHits>>()?
                        .get_message(*id)
                        .map(|(hits, _)| hits)
                })
                .filter(|hits| hits.pointer == pointer)
                .flat_map(|hits| {
                    hits.picks
                        .iter()
                        .map(move |(entity, hit)| (hits.order, *entity, hit.clone()))
                })
                .collect::<Vec<_>>();
            ordered.sort_by(|(layer_a, _, a), (layer_b, _, b)| {
                bevy::math::FloatOrd(*layer_b)
                    .cmp(&bevy::math::FloatOrd(*layer_a))
                    .then(bevy::math::FloatOrd(a.depth).cmp(&bevy::math::FloatOrd(b.depth)))
            });
            for (_, entity, hit) in ordered {
                if !world.entities().contains(entity) {
                    continue;
                }
                let pickable = world.get::<Pickable>(entity).copied().unwrap_or_default();
                if pickable.is_hoverable {
                    resolved.insert(entity, hit);
                }
                if pickable.should_block_lower {
                    break;
                }
            }
        }
    }
    *world.resource_mut::<HoverMap>().entry(pointer).or_default() = resolved;
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy::picking::{backend::HitData, pointer::Location};

    #[test]
    fn refresh_uses_current_backend_hits_and_honors_capture_and_pickable_order() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<PointerHits>()
            .init_resource::<bevy::picking::pointer::PointerMap>()
            .init_resource::<HoverMap>()
            .init_resource::<PointerCaptureMap>();
        install(&mut app);
        let id = PointerId::Custom(Default::default());
        app.add_systems(
            PreUpdate,
            (move |world: &mut World| refresh_hover(world, id))
                .in_set(PickingSystems::Hover)
                .after(capture_hits)
                .before(mark_dispatch),
        );
        app.world_mut().spawn((
            id,
            PointerLocation {
                location: Some(Location {
                    target: NormalizedRenderTarget::None {
                        width: 100,
                        height: 100,
                    },
                    position: Vec2::ONE,
                }),
            },
        ));
        let ignored = app.world_mut().spawn(Pickable::IGNORE).id();
        let pass = app
            .world_mut()
            .spawn(Pickable {
                is_hoverable: true,
                should_block_lower: false,
            })
            .id();
        let front = app.world_mut().spawn_empty().id();
        let back = app.world_mut().spawn_empty().id();
        let hit = HitData::new(Entity::PLACEHOLDER, 1.0, None, None);
        app.world_mut()
            .write_message(PointerHits::new(id, vec![(back, hit.clone())], 0.0));
        app.world_mut().write_message(PointerHits::new(
            id,
            vec![
                (ignored, hit.clone()),
                (pass, hit.clone()),
                (front, hit.clone()),
            ],
            1.0,
        ));
        app.update();
        let hovered = &app.world().resource::<HoverMap>()[&id];
        assert_eq!(hovered.len(), 2);
        assert!(hovered.contains_key(&pass) && hovered.contains_key(&front));
        app.world_mut()
            .resource_mut::<PointerCaptureMap>()
            .capture(id, back, hit);
        app.update();
        assert_eq!(
            app.world().resource::<HoverMap>()[&id]
                .keys()
                .copied()
                .collect::<Vec<_>>(),
            [back]
        );
        app.world_mut()
            .resource_mut::<PointerCaptureMap>()
            .release(id);
        app.update();
        assert!(app.world().resource::<HoverMap>()[&id].is_empty());
    }
}
