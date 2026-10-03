use bevy::{
    picking::{
        PickingSystems,
        hover::HoverMap,
        pointer::{PointerAction, PointerId, PointerInput},
    },
    prelude::*,
    time::Real,
    window::RequestRedraw,
};
use bevy_widgetry_log::widgetry_info;
use std::time::Duration;

const COLD_WARMUP: Duration = Duration::from_millis(200);
const WARM_DELAY: Duration = Duration::from_millis(50);
const COOLDOWN: Duration = Duration::from_millis(300);

#[derive(Component, Default, Clone)]
pub(crate) struct Tooltip;

pub(crate) struct TooltipPlugin;

#[derive(Resource)]
pub(crate) struct TooltipState {
    candidate: Option<Entity>,
    visible: Option<Entity>,
    show_elapsed: Duration,
    show_delay: Duration,
    cooldown_remaining: Duration,
}

impl Default for TooltipState {
    fn default() -> Self {
        Self {
            candidate: None,
            visible: None,
            show_elapsed: Duration::ZERO,
            show_delay: COLD_WARMUP,
            cooldown_remaining: Duration::ZERO,
        }
    }
}

#[derive(EntityEvent)]
pub(crate) struct ShowTooltip {
    #[event_target]
    pub(crate) source: Entity,
}

#[derive(EntityEvent)]
pub(crate) struct HideTooltip {
    #[event_target]
    pub(crate) source: Entity,
}

fn resolve_anchor(
    hover_map: &HoverMap,
    parents: &Query<&ChildOf>,
    tooltips: &Query<(), With<Tooltip>>,
) -> Option<Entity> {
    let hovered = hover_map.get(&PointerId::Mouse)?;
    let target = hovered
        .iter()
        .min_by(|(_, left), (_, right)| left.depth.total_cmp(&right.depth))?
        .0;
    if tooltips.contains(*target) {
        return Some(*target);
    }
    parents
        .iter_ancestors(*target)
        .find(|&ancestor| tooltips.contains(ancestor))
}

fn update_tooltip(
    real_time: Res<Time<Real>>,
    hover_map: Res<HoverMap>,
    parents: Query<&ChildOf>,
    tooltips: Query<(), With<Tooltip>>,
    mut pointer_inputs: MessageReader<PointerInput>,
    mut state: ResMut<TooltipState>,
    mut redraw: MessageWriter<RequestRedraw>,
    mut commands: Commands,
) {
    state.cooldown_remaining = state.cooldown_remaining.saturating_sub(real_time.delta());
    let mut pointer_moved = false;
    for input in pointer_inputs.read() {
        pointer_moved |= input.pointer_id == PointerId::Mouse
            && matches!(input.action, PointerAction::Move { .. });
    }
    let resolved = resolve_anchor(&hover_map, &parents, &tooltips);

    if let Some(visible) = state.visible
        && resolved != Some(visible)
    {
        commands.trigger(HideTooltip { source: visible });
        state.visible = None;
        state.cooldown_remaining = COOLDOWN;
        state.candidate = resolved;
        state.show_elapsed = Duration::ZERO;
        state.show_delay = WARM_DELAY;
        if resolved.is_some() {
            redraw.write(RequestRedraw);
        }
        return;
    }

    if state.visible.is_some() {
        return;
    }

    if state.candidate != resolved {
        state.candidate = resolved;
        state.show_elapsed = Duration::ZERO;
        state.show_delay = if state.cooldown_remaining.is_zero() {
            COLD_WARMUP
        } else {
            WARM_DELAY
        };
        if resolved.is_some() {
            redraw.write(RequestRedraw);
        }
        return;
    }

    let Some(candidate) = state.candidate else {
        state.show_elapsed = Duration::ZERO;
        return;
    };
    if pointer_moved {
        state.show_elapsed = Duration::ZERO;
        redraw.write(RequestRedraw);
        return;
    }

    state.show_elapsed += real_time.delta();
    if state.show_elapsed >= state.show_delay {
        commands.trigger(ShowTooltip { source: candidate });
        state.visible = Some(candidate);
        state.show_elapsed = Duration::ZERO;
    } else {
        redraw.write(RequestRedraw);
    }
}

fn tooltip_removed(
    event: On<Remove, Tooltip>,
    mut state: ResMut<TooltipState>,
    mut commands: Commands,
) {
    let anchor = event.entity;
    if state.candidate == Some(anchor) {
        state.candidate = None;
        state.show_elapsed = Duration::ZERO;
    }
    if state.visible == Some(anchor) {
        state.visible = None;
        state.cooldown_remaining = COOLDOWN;
        commands.trigger(HideTooltip { source: anchor });
    }
}

impl Plugin for TooltipPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TooltipState>()
            .add_message::<RequestRedraw>()
            .add_observer(tooltip_removed)
            .add_systems(PreUpdate, update_tooltip.in_set(PickingSystems::PostHover));
        widgetry_info!("TooltipPlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败；生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::{
        camera::NormalizedRenderTarget,
        ecs::entity::EntityHashMap,
        picking::{
            backend::HitData,
            pointer::{Location, PointerLocation},
        },
        time::{TimeUpdateStrategy, Virtual},
    };
    use bevy_widgetry_test_utils::scene_app;

    #[derive(Resource, Default)]
    struct Events {
        shown: Vec<Entity>,
        hidden: Vec<Entity>,
    }

    fn record_show(event: On<ShowTooltip>, mut events: ResMut<Events>) {
        events.shown.push(event.source);
    }

    fn record_hide(event: On<HideTooltip>, mut events: ResMut<Events>) {
        events.hidden.push(event.source);
    }

    fn test_app() -> App {
        let mut app = scene_app();
        app.init_resource::<HoverMap>()
            .init_resource::<Events>()
            .add_message::<PointerInput>()
            .add_plugins(TooltipPlugin)
            .add_observer(record_show)
            .add_observer(record_hide);
        app.world_mut().spawn((
            PointerId::Mouse,
            PointerLocation::new(Location {
                target: NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ZERO,
            }),
        ));
        app
    }

    fn hover(app: &mut App, target: Option<Entity>) {
        let mut map = app.world_mut().resource_mut::<HoverMap>();
        map.clear();
        if let Some(target) = target {
            let mut hits = EntityHashMap::default();
            hits.insert(target, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
            map.insert(PointerId::Mouse, hits);
        }
    }

    fn advance(app: &mut App, duration: Duration) {
        *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
            TimeUpdateStrategy::ManualDuration(duration);
        app.update();
    }

    #[test]
    fn resolves_anchor_and_descendant_after_cold_delay() {
        for descendant in [false, true] {
            let mut app = test_app();
            let anchor = app.world_mut().spawn(Tooltip).id();
            let target = if descendant {
                let child = app.world_mut().spawn_empty().id();
                app.world_mut().entity_mut(anchor).add_child(child);
                child
            } else {
                anchor
            };
            hover(&mut app, Some(target));
            advance(&mut app, Duration::ZERO);
            advance(&mut app, Duration::from_millis(199));
            assert!(app.world().resource::<Events>().shown.is_empty());
            advance(&mut app, Duration::from_millis(1));
            assert_eq!(app.world().resource::<Events>().shown, vec![anchor]);
        }
    }

    #[test]
    fn waiting_candidate_requests_redraw_until_shown() {
        let mut app = test_app();
        let anchor = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(anchor));

        advance(&mut app, Duration::ZERO);
        assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();

        advance(&mut app, Duration::from_millis(199));
        assert!(!app.world().resource::<Messages<RequestRedraw>>().is_empty());
        app.world_mut()
            .resource_mut::<Messages<RequestRedraw>>()
            .clear();

        advance(&mut app, Duration::from_millis(1));
        assert_eq!(app.world().resource::<Events>().shown, vec![anchor]);
        assert!(app.world().resource::<Messages<RequestRedraw>>().is_empty());
    }

    #[test]
    fn paused_virtual_time_does_not_stop_show_delay() {
        let mut app = test_app();
        app.world_mut().resource_mut::<Time<Virtual>>().pause();
        let anchor = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(anchor));

        advance(&mut app, Duration::ZERO);
        advance(&mut app, COLD_WARMUP);

        assert_eq!(app.world().resource::<Events>().shown, vec![anchor]);
        assert!(app.world().resource::<Time<Virtual>>().is_paused());
    }

    #[test]
    fn ignores_disabled_state_but_not_missing_ancestor() {
        let mut app = test_app();
        let plain = app.world_mut().spawn_empty().id();
        hover(&mut app, Some(plain));
        advance(&mut app, Duration::from_millis(500));
        assert!(app.world().resource::<Events>().shown.is_empty());

        let disabled = app
            .world_mut()
            .spawn((Tooltip, bevy::ui::InteractionDisabled))
            .id();
        hover(&mut app, Some(disabled));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, COLD_WARMUP);
        assert_eq!(app.world().resource::<Events>().shown, vec![disabled]);
    }

    #[test]
    fn pointer_movement_resets_show_timer() {
        let mut app = test_app();
        let anchor = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(anchor));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, Duration::from_millis(150));
        let target = NormalizedRenderTarget::None {
            width: 100,
            height: 100,
        };
        let mut pointer_inputs = app.world_mut().resource_mut::<Messages<PointerInput>>();
        pointer_inputs.write(PointerInput::new(
            PointerId::Mouse,
            Location {
                target: target.clone(),
                position: Vec2::X,
            },
            PointerAction::Move { delta: Vec2::X },
        ));
        pointer_inputs.write(PointerInput::new(
            PointerId::Mouse,
            Location {
                target,
                position: Vec2::ZERO,
            },
            PointerAction::Move { delta: Vec2::NEG_X },
        ));
        advance(&mut app, Duration::from_millis(50));
        advance(&mut app, Duration::from_millis(199));
        assert!(app.world().resource::<Events>().shown.is_empty());
        advance(&mut app, Duration::from_millis(1));
        assert_eq!(app.world().resource::<Events>().shown, vec![anchor]);
    }

    #[test]
    fn visible_anchor_stays_open_and_switches_warm() {
        let mut app = test_app();
        let a = app.world_mut().spawn(Tooltip).id();
        let a_child = app.world_mut().spawn_empty().id();
        app.world_mut().entity_mut(a).add_child(a_child);
        let b = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(a));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, COLD_WARMUP);

        hover(&mut app, Some(a_child));
        advance(&mut app, Duration::from_millis(1));
        assert!(app.world().resource::<Events>().hidden.is_empty());

        hover(&mut app, Some(b));
        advance(&mut app, Duration::ZERO);
        assert_eq!(app.world().resource::<Events>().hidden, vec![a]);
        advance(&mut app, Duration::from_millis(49));
        assert_eq!(app.world().resource::<Events>().shown, vec![a]);
        advance(&mut app, Duration::from_millis(1));
        assert_eq!(app.world().resource::<Events>().shown, vec![a, b]);
    }

    #[test]
    fn cooldown_expires_back_to_cold_delay() {
        let mut app = test_app();
        let a = app.world_mut().spawn(Tooltip).id();
        let b = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(a));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, COLD_WARMUP);
        hover(&mut app, None);
        advance(&mut app, Duration::ZERO);
        // 响应式 App 可能整个 cooldown 都没有 update；Real time 不受 Virtual time 的 250ms 截断。
        advance(&mut app, COOLDOWN);
        assert!(
            app.world()
                .resource::<TooltipState>()
                .cooldown_remaining
                .is_zero()
        );
        hover(&mut app, Some(b));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, Duration::from_millis(199));
        assert_eq!(app.world().resource::<Events>().shown, vec![a]);
        advance(&mut app, Duration::from_millis(1));
        assert_eq!(app.world().resource::<Events>().shown, vec![a, b]);
    }

    #[test]
    fn candidate_entering_near_cooldown_end_keeps_warm_delay() {
        let mut app = test_app();
        let a = app.world_mut().spawn(Tooltip).id();
        let b = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(a));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, COLD_WARMUP);
        hover(&mut app, None);
        advance(&mut app, Duration::ZERO);
        advance(&mut app, Duration::from_millis(250));
        advance(&mut app, Duration::from_millis(30));

        hover(&mut app, Some(b));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, Duration::from_millis(20));
        assert_eq!(app.world().resource::<Events>().shown, vec![a]);
        advance(&mut app, Duration::from_millis(30));
        assert_eq!(app.world().resource::<Events>().shown, vec![a, b]);
    }

    #[test]
    fn removing_visible_tooltip_hides_it() {
        let mut app = test_app();
        let anchor = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(anchor));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, COLD_WARMUP);
        app.world_mut().entity_mut(anchor).remove::<Tooltip>();
        app.world_mut().flush();
        assert_eq!(app.world().resource::<Events>().hidden, vec![anchor]);
        assert!(app.world().resource::<TooltipState>().visible.is_none());
    }
    #[test]
    fn destroyed_candidate_is_cleared() {
        let mut app = test_app();
        let anchor = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(anchor));
        advance(&mut app, Duration::ZERO);
        app.world_mut().despawn(anchor);
        advance(&mut app, COLD_WARMUP);
        assert!(app.world().resource::<TooltipState>().candidate.is_none());
        assert!(app.world().resource::<Events>().shown.is_empty());
        assert!(app.world().resource::<Events>().hidden.is_empty());
    }

    #[test]
    fn front_hit_owns_tooltip_resolution() {
        let mut app = test_app();
        let front = app.world_mut().spawn(Tooltip).id();
        let back = app.world_mut().spawn(Tooltip).id();
        let blocker = app.world_mut().spawn_empty().id();
        for (target, expected) in [(blocker, None), (front, Some(front))] {
            let mut hits = EntityHashMap::default();
            hits.insert(target, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
            hits.insert(back, HitData::new(Entity::PLACEHOLDER, 2.0, None, None));
            app.world_mut()
                .resource_mut::<HoverMap>()
                .insert(PointerId::Mouse, hits);
            advance(&mut app, Duration::ZERO);
            advance(&mut app, COLD_WARMUP);
            assert_eq!(
                app.world().resource::<Events>().shown,
                expected.into_iter().collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn touch_movement_does_not_reset_mouse_candidate() {
        let mut app = test_app();
        let anchor = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(anchor));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, Duration::from_millis(150));
        app.world_mut().write_message(PointerInput::new(
            PointerId::Touch(7),
            Location {
                target: NormalizedRenderTarget::None {
                    width: 100,
                    height: 100,
                },
                position: Vec2::ONE,
            },
            PointerAction::Move { delta: Vec2::ONE },
        ));
        advance(&mut app, Duration::from_millis(50));
        assert_eq!(app.world().resource::<Events>().shown, vec![anchor]);
    }
}
