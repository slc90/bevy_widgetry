use bevy::{
    camera::NormalizedRenderTarget,
    ecs::{
        entity::{ContainsEntity, EntityHashSet},
        schedule::{ScheduleCleanupPolicy, ScheduleError},
        system::SystemParam,
    },
    picking::{
        PickingSystems,
        events::pointer_events,
        hover::{
            DirectlyHovered, HoverMap, Hovered, update_interactions, update_is_directly_hovered,
            update_is_hovered,
        },
        pointer::{Location, PointerId, PointerLocation},
    },
    prelude::*,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};

pub struct WidgetryPointerPlugin;

#[derive(SystemParam)]
pub struct WidgetryPointerQuery<'w, 's> {
    pointers: Query<'w, 's, (&'static PointerId, &'static PointerLocation)>,
    windows: Query<'w, 's, (), With<Window>>,
}

impl WidgetryPointerQuery<'_, '_> {
    pub fn location(&self, id: PointerId) -> Option<&Location> {
        self.iter()
            .find_map(|(pointer, location)| (pointer == id).then_some(location))
    }

    pub fn iter(&self) -> impl Iterator<Item = (PointerId, &Location)> {
        self.pointers.iter().filter_map(|(id, pointer)| {
            let location = pointer.location.as_ref()?;
            let target_exists = match &location.target {
                NormalizedRenderTarget::Window(window) => self.windows.contains(window.entity()),
                _ => true,
            };
            (location.position.is_finite() && target_exists).then_some((*id, location))
        })
    }
}

impl Plugin for WidgetryPointerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, replace_hover_writers).add_systems(
            PreUpdate,
            update_hover
                .in_set(PickingSystems::Hover)
                .after(update_interactions)
                .before(pointer_events),
        );
        widgetry_info!("WidgetryPointerPlugin 注册完成");
    }
}

fn replace_hover_writers(world: &mut World) -> Result {
    world.schedule_scope(PreUpdate, |world, schedule| -> Result {
        for result in [
            schedule.remove_systems_in_set(
                update_is_hovered,
                world,
                ScheduleCleanupPolicy::RemoveSystemsOnly,
            ),
            schedule.remove_systems_in_set(
                update_is_directly_hovered,
                world,
                ScheduleCleanupPolicy::RemoveSystemsOnly,
            ),
        ] {
            check_replacement(result)?;
        }
        Ok(())
    })
}

fn check_replacement(result: std::result::Result<usize, ScheduleError>) -> Result {
    match result {
        Ok(_) | Err(ScheduleError::SetNotFound) => Ok(()),
        Err(error) => {
            widgetry_error!(%error, "Pointer hover writer 替换失败");
            Err(crate::scene::logged_error(format!(
                "Pointer hover writer replacement failed: {error}"
            )))
        }
    }
}

fn update_hover(
    map: Option<Res<HoverMap>>,
    pointers: WidgetryPointerQuery,
    entities: Query<()>,
    parents: Query<&ChildOf>,
    hovered: Query<(Entity, &Hovered)>,
    directly_hovered: Query<(Entity, &DirectlyHovered)>,
    mut direct: Local<EntityHashSet>,
    mut ancestors: Local<EntityHashSet>,
    mut commands: Commands,
) {
    let Some(map) = map else {
        return;
    };
    direct.clear();
    ancestors.clear();
    for (id, _) in pointers.iter() {
        if let Some(hits) = map.get(&id) {
            for &hit in hits.keys().filter(|&&hit| entities.contains(hit)) {
                direct.insert(hit);
                let mut entity = hit;
                while ancestors.insert(entity) {
                    let Ok(parent) = parents.get(entity) else {
                        break;
                    };
                    entity = parent.parent();
                }
            }
        }
    }
    for (entity, value) in &hovered {
        let next = ancestors.contains(&entity);
        if value.0 != next {
            commands.entity(entity).insert(Hovered(next));
        }
    }
    for (entity, value) in &directly_hovered {
        let next = direct.contains(&entity);
        if value.0 != next {
            commands.entity(entity).insert(DirectlyHovered(next));
        }
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::LogCapture;

    #[test]
    fn replacement_failure_logs_and_propagates_error_while_missing_writer_is_normal() {
        let logs = LogCapture::default();
        logs.run(|| {
            assert!(check_replacement(Ok(1)).is_ok());
            assert!(check_replacement(Err(ScheduleError::SetNotFound)).is_ok());
            assert!(logs.records().is_empty());
            let error = check_replacement(Err(ScheduleError::ScheduleNotFound)).unwrap_err();
            assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
            let records = logs.records();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].level, bevy::log::Level::ERROR);
            assert!(records[0].fields.contains_key("error"));
        });
    }
}
