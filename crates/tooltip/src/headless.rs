use bevy::{
    picking::{
        PickingSystems,
        hover::HoverMap,
        pointer::{Location, PointerId, PointerLocation},
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

/// 标识可由 hover ancestor lookup 解析的 Tooltip anchor。
#[derive(Component, Default, Clone)]
pub(crate) struct Tooltip;

/// 管理 hover resolution、show delay 与 hide cooldown 的内部 plugin。
pub(crate) struct TooltipPlugin;

/// 当前唯一 candidate、visible anchor 及 timing state。
#[derive(Resource)]
pub(crate) struct TooltipState {
    /// 正在等待 show delay 的 anchor。
    candidate: Option<Entity>,
    /// 已经发出 show event 且尚未 hide 的 anchor。
    visible: Option<Entity>,
    /// 当前 candidate 在最后一次 pointer movement 后累计的停留时间。
    show_elapsed: Duration,
    /// candidate 建立时锁定的 warm 或 cold delay，不随 cooldown 后续归零而改变。
    show_delay: Duration,
    /// 最近一次 hide 后剩余的 warm mode 时间。
    cooldown_remaining: Duration,
    /// 用于识别 pointer movement 的上一帧 mouse location。
    pointer_location: Option<Location>,
}

impl Default for TooltipState {
    fn default() -> Self {
        Self {
            candidate: None,
            visible: None,
            show_elapsed: Duration::ZERO,
            show_delay: COLD_WARMUP,
            cooldown_remaining: Duration::ZERO,
            pointer_location: None,
        }
    }
}

/// 请求 styled layer 为 anchor 创建 popup。
#[derive(EntityEvent)]
pub(crate) struct ShowTooltip {
    /// Tooltip anchor，不是 popup entity。
    #[event_target]
    pub(crate) source: Entity,
}

/// 请求 styled layer 销毁 anchor 的 popup。
#[derive(EntityEvent)]
pub(crate) struct HideTooltip {
    /// Tooltip anchor，不是 popup entity。
    #[event_target]
    pub(crate) source: Entity,
}

/// 从最前方 mouse hover target 沿 ancestor 向上解析第一个 Tooltip anchor。
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

/// 返回 mouse pointer 的当前 location；没有 active mouse 时视为未发生移动。
fn mouse_location(pointers: &Query<(&PointerId, &PointerLocation)>) -> Option<Location> {
    pointers
        .iter()
        .find(|(id, _)| **id == PointerId::Mouse)
        .and_then(|(_, location)| location.location().cloned())
}

/// 在 PostHover 中统一推进 resolution、warmup、cooldown 与 show/hide event。
fn update_tooltip(
    real_time: Res<Time<Real>>,
    hover_map: Res<HoverMap>,
    parents: Query<&ChildOf>,
    tooltips: Query<(), With<Tooltip>>,
    pointers: Query<(&PointerId, &PointerLocation)>,
    mut state: ResMut<TooltipState>,
    mut redraw: MessageWriter<RequestRedraw>,
    mut commands: Commands,
) {
    state.cooldown_remaining = state.cooldown_remaining.saturating_sub(real_time.delta());
    let pointer_location = mouse_location(&pointers);
    let pointer_moved =
        state.pointer_location.is_some() && state.pointer_location != pointer_location;
    state.pointer_location = pointer_location;
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

/// Tooltip marker 被移除或 anchor despawn 时同步清除 candidate、visible 与 popup。
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::{
        camera::NormalizedRenderTarget,
        ecs::entity::EntityHashMap,
        picking::backend::HitData,
        time::{TimeUpdateStrategy, Virtual},
    };
    use bevy_widgetry_test_utils::scene_app;

    /// 记录 headless show/hide event，测试不依赖 styled popup observer。
    #[derive(Resource, Default)]
    struct Events {
        /// 按发生顺序保存 show anchor。
        shown: Vec<Entity>,
        /// 按发生顺序保存 hide anchor。
        hidden: Vec<Entity>,
    }

    /// 收集 show event 的 anchor identity。
    fn record_show(event: On<ShowTooltip>, mut events: ResMut<Events>) {
        events.shown.push(event.source);
    }

    /// 收集 hide event 的 anchor identity。
    fn record_hide(event: On<HideTooltip>, mut events: ResMut<Events>) {
        events.hidden.push(event.source);
    }

    /// 构造带 mouse location、HoverMap 与 headless plugin 的确定性 timing App。
    fn test_app() -> App {
        let mut app = scene_app();
        app.init_resource::<HoverMap>()
            .init_resource::<Events>()
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

    /// 替换 mouse 当前唯一 hover target，depth 固定为最前方。
    fn hover(app: &mut App, target: Option<Entity>) {
        let mut map = app.world_mut().resource_mut::<HoverMap>();
        map.clear();
        if let Some(target) = target {
            let mut hits = EntityHashMap::default();
            hits.insert(target, HitData::new(Entity::PLACEHOLDER, 0.0, None, None));
            map.insert(PointerId::Mouse, hits);
        }
    }

    /// 用固定 delta 推进一帧，避免现实时间影响 timing 断言。
    fn advance(app: &mut App, duration: Duration) {
        *app.world_mut().resource_mut::<TimeUpdateStrategy>() =
            TimeUpdateStrategy::ManualDuration(duration);
        app.update();
    }

    /// 直接 hover anchor 与 hover descendant 都应解析到同一 Tooltip，首次必须完整等待 200ms。
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

    /// 响应式 App 等待 show delay 时必须持续请求 redraw，显示后立即停止。
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

    /// Tooltip UI timing 使用现实时间；暂停游戏 Virtual time 不得阻止显示。
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

    /// 无 Tooltip ancestor 时不产生 event；InteractionDisabled anchor 仍按正常 timing 显示。
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

    /// show 前发生 pointer movement 必须重置 cold timer，不能累计移动前的停留时间。
    #[test]
    fn pointer_movement_resets_show_timer() {
        let mut app = test_app();
        let anchor = app.world_mut().spawn(Tooltip).id();
        hover(&mut app, Some(anchor));
        advance(&mut app, Duration::ZERO);
        advance(&mut app, Duration::from_millis(150));
        let mut query = app.world_mut().query::<&mut PointerLocation>();
        let mut location = query.single_mut(app.world_mut()).unwrap();
        location.location.as_mut().unwrap().position.x = 1.0;
        advance(&mut app, Duration::from_millis(50));
        advance(&mut app, Duration::from_millis(199));
        assert!(app.world().resource::<Events>().shown.is_empty());
        advance(&mut app, Duration::from_millis(1));
        assert_eq!(app.world().resource::<Events>().shown, vec![anchor]);
    }

    /// visible Tooltip 内部移动保持显示；切换到 B 立即 hide A，并在 warm 50ms 后 show B。
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

    /// hide 后 300ms 内使用 warm delay，cooldown 结束后新 candidate 恢复 cold 200ms。
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

    /// candidate 在 cooldown 尾部建立后锁定 warm delay，即使等待期间 cooldown 已归零也仍按 50ms 显示。
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

    /// 移除 visible marker 必须立即清理 state 并发送唯一 hide event。
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
}
