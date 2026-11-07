use crate::headless::{ShowTooltip, WidgetryTooltipHeadless, WidgetryTooltipHeadlessPlugin};
use crate::{TooltipError, TooltipFailure, error::failure};
use bevy::{
    prelude::*,
    ui::OverrideClip,
    ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverPlugin, PopoverSide},
};
use bevy_widgetry_core::{foreground::ResolvedForeground, ui::WidgetryUiPlugin, z_index};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState},
};
use bevy_widgetry_internal_log::widgetry_debug;
use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged, WidgetryThemePlugin};
use std::sync::Arc;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryTooltipProps)]
#[require(crate::colors::ColorState)]
pub struct WidgetryTooltip;

pub struct WidgetryTooltipProps {
    pub colors: crate::WidgetryTooltipColorOverrides,
    pub content: TooltipContentFactory,
}

#[derive(Clone)]
pub struct TooltipContentFactory(Option<Arc<dyn Fn() -> Box<dyn SceneList> + Send + Sync>>);

#[derive(Component)]
struct TooltipContent(TooltipContentFactory);

#[derive(Component, Default, Clone)]
#[require(PopupFailure)]
pub(crate) struct TooltipPopup;

#[derive(Component, Default)]
struct PopupFailure(
    FailureState<(
        Entity,
        std::mem::Discriminant<bevy::ecs::query::QueryEntityError>,
    )>,
);

pub struct WidgetryTooltipPlugin;

impl TooltipContentFactory {
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn() -> S + Send + Sync + 'static,
    {
        Self(Some(Arc::new(move || Box::new(factory()))))
    }

    fn build(&self, anchor: Entity) -> Result<Box<dyn SceneList>, TooltipFailure> {
        let Some(factory) = &self.0 else {
            return Err(failure(
                TooltipError::MissingContent { anchor },
                ErrorFrame::new("tooltip.build_content"),
            ));
        };
        Ok(factory())
    }
}

impl Default for WidgetryTooltipProps {
    fn default() -> Self {
        Self {
            content: TooltipContentFactory(None),
            colors: default(),
        }
    }
}

impl WidgetryTooltip {
    fn scene(props: WidgetryTooltipProps) -> impl Scene {
        let missing_content = props.content.0.is_none();
        bsn! {
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(TooltipFailure::into_bevy_error))
            template(move |context| {
                if missing_content {
                    return Err(failure(TooltipError::MissingContent {anchor:context.entity.id()}, ErrorFrame::new("tooltip.initialize")).into_bevy_error());
                }
                Ok(WidgetryTooltipHeadless)
            })
            template(move |_| Ok(TooltipContent(props.content.clone())))
        }
    }
}

fn popup_scene(
    anchor: Entity,
    content: Box<dyn SceneList>,
    colors: bevy_widgetry_theme::WidgetryTooltipColors,
) -> impl Scene {
    let colors = colors.popup.normal;
    bsn! {
        TooltipPopup
        template(move |_| Ok(ChildOf(anchor)))
        Popover {
            positions: {vec![
                PopoverPlacement { side: PopoverSide::Bottom, align: PopoverAlign::Center, gap: 6.0 },
                PopoverPlacement { side: PopoverSide::Top, align: PopoverAlign::Center, gap: 6.0 },
                PopoverPlacement { side: PopoverSide::Right, align: PopoverAlign::Center, gap: 6.0 },
                PopoverPlacement { side: PopoverSide::Left, align: PopoverAlign::Center, gap: 6.0 },
            ]},
            window_margin: 8.0,
        }
        OverrideClip
        GlobalZIndex({z_index::TOOLTIP})
        Pickable::IGNORE
        template(move |_| Ok(BackgroundColor(colors.background)))
        template(move |_| Ok(BorderColor::all(colors.border)))
        template(move |_| Ok(ResolvedForeground(colors.foreground)))
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(8), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
        }
        Children [{content}]
    }
}

fn show_tooltip(event: On<ShowTooltip>, mut commands: Commands) {
    let show = *event;
    commands.queue(move |world: &mut World| -> Result {
        if !show.is_current(world)
            || show.popup(world).is_some()
            || world.get::<WidgetryTooltip>(show.source).is_none()
        {
            return Ok(());
        }
        let anchor = show.source;
        let Some(content) = world.get::<TooltipContent>(anchor) else {
            return Ok(());
        };
        let content = content.0.clone();
        let overrides = world
            .get::<crate::colors::ColorState>(anchor)
            .ok_or_else(|| {
                failure(
                    TooltipError::AnchorColors {
                        anchor,
                        popup: None,
                        source: None,
                    },
                    ErrorFrame::new("tooltip.show"),
                )
                .into_bevy_error()
            })?;
        let colors = overrides
            .0
            .resolve(&world.resource::<WidgetryTheme>().tooltip);
        let content = content
            .build(anchor)
            .map_err(TooltipFailure::into_bevy_error)?;
        let popup = world.spawn_empty().id();
        let result = bevy_widgetry_core::scene::apply_scene(
            &mut world.entity_mut(popup),
            popup_scene(anchor, content, colors),
        );
        if let Err(error) = result {
            world.despawn(popup);
            return Err(error
                .with_context(
                    ErrorFrame::new("tooltip.show")
                        .field("anchor", anchor)
                        .field("popup", popup),
                )
                .into_bevy_error());
        }
        show.register_popup(world, popup)
            .map_err(TooltipFailure::into_bevy_error)?;
        Ok(())
    });
}
fn apply_tooltip_colors(
    colors: &bevy_widgetry_theme::WidgetryTooltipColors,
    (mut background, mut border, mut foreground): (
        Mut<BackgroundColor>,
        Mut<BorderColor>,
        Mut<ResolvedForeground>,
    ),
) {
    #[cfg(test)]
    STYLE_WORK.with(|work| work.set(work.get() + 1));
    let state = colors.popup.normal;
    background.set_if_neq(BackgroundColor(state.background));
    border.set_if_neq(BorderColor::all(state.border));
    foreground.set_if_neq(ResolvedForeground(state.foreground));
}

#[cfg(test)]
thread_local! {
    static STYLE_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
fn popup_color_failure(
    state: &mut PopupFailure,
    anchor: Entity,
    popup: Entity,
    source: bevy::ecs::query::QueryEntityError,
    operation: &'static str,
    batch: &mut DiagnosticBatch,
) {
    let error = state
        .0
        .failure((anchor, std::mem::discriminant(&source)), || {
            Arc::new(failure(
                TooltipError::AnchorColors {
                    anchor,
                    popup: Some(popup),
                    source: Some(source),
                },
                ErrorFrame::new("tooltip.popup_colors"),
            ))
        })
        .with_context(
            ErrorFrame::new(operation)
                .field("anchor", anchor)
                .field("popup", popup),
        );
    for diagnostic in error.shared_diagnostics() {
        batch.push_shared(diagnostic);
    }
}

fn refresh_tooltip_theme(
    _: On<WidgetryThemeChanged>,
    mode: Res<WidgetryTheme>,
    anchors: Query<&crate::colors::ColorState, With<WidgetryTooltip>>,
    mut popups: Query<
        (
            Entity,
            &ChildOf,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut ResolvedForeground,
            &mut PopupFailure,
        ),
        With<TooltipPopup>,
    >,
) -> Result<(), BevyError> {
    let mut batch = DiagnosticBatch::default();
    for (popup, parent, background, border, foreground, mut failure) in &mut popups {
        let overrides = match anchors.get(parent.parent()) {
            Ok(overrides) => overrides,
            Err(source) => {
                popup_color_failure(
                    &mut failure,
                    parent.parent(),
                    popup,
                    source,
                    "tooltip.refresh_theme",
                    &mut batch,
                );
                continue;
            }
        };
        failure.0.recover(|| {
            ErrorFrame::new("tooltip.popup_colors")
                .field("anchor", parent.parent())
                .field("popup", popup)
        });
        apply_tooltip_colors(
            &overrides.0.resolve(&mode.tooltip),
            (background, border, foreground),
        );
    }
    match batch.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn update_tooltip_colors(
    mode: Res<WidgetryTheme>,
    anchors: Query<Ref<crate::colors::ColorState>, With<WidgetryTooltip>>,
    mut popups: Query<
        (
            Entity,
            Ref<ChildOf>,
            &mut BackgroundColor,
            &mut BorderColor,
            &mut ResolvedForeground,
            Ref<TooltipPopup>,
            &mut PopupFailure,
        ),
        With<TooltipPopup>,
    >,
) -> Result<(), BevyError> {
    let mut batch = DiagnosticBatch::default();
    for (entity, parent, background, border, foreground, popup, mut failure) in &mut popups {
        let overrides = match anchors.get(parent.parent()) {
            Ok(overrides) => overrides,
            Err(source) => {
                popup_color_failure(
                    &mut failure,
                    parent.parent(),
                    entity,
                    source,
                    "tooltip.update_colors",
                    &mut batch,
                );
                continue;
            }
        };
        let recovered = failure.0.recover(|| {
            ErrorFrame::new("tooltip.popup_colors")
                .field("anchor", parent.parent())
                .field("popup", entity)
        });
        if !recovered
            && !popup.is_added()
            && !parent.is_changed()
            && !overrides.is_changed()
            && !background.is_added()
            && !border.is_added()
            && !foreground.is_added()
        {
            continue;
        }
        apply_tooltip_colors(
            &overrides.0.resolve(&mode.tooltip),
            (background, border, foreground),
        );
    }
    match batch.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

impl Plugin for WidgetryTooltipPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.is_plugin_added::<WidgetryTooltipHeadlessPlugin>() {
            app.add_plugins(WidgetryTooltipHeadlessPlugin);
        }
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }
        if !app.is_plugin_added::<WidgetryThemePlugin>() {
            app.add_plugins(WidgetryThemePlugin);
        }
        app.add_systems(
            PostUpdate,
            update_tooltip_colors.in_set(bevy_widgetry_core::ui::WidgetryUiSystems::Colors),
        );
        app.add_observer(show_tooltip)
            .add_observer(refresh_tooltip_theme);
        widgetry_debug!("WidgetryTooltipPlugin 注册完成");
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    use bevy_widgetry_test_utils::{LogCapture, scene_app};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Component, Default, Clone)]
    struct ContentMarker;

    #[test]
    fn missing_runtime_content_has_the_same_classification_with_independent_identity() {
        use bevy_widgetry_test_utils::DiagnosticPhase;
        let anchor = Entity::PLACEHOLDER;
        let logs = LogCapture::default();
        logs.run(|| {
            let factory = TooltipContentFactory(None);
            let first = factory.build(anchor).err().unwrap();
            let second = factory.build(anchor).err().unwrap();
            assert!(matches!(first.kind(), TooltipError::MissingContent {anchor: actual} if *actual == anchor));
            assert_ne!(first.origin().id(), second.origin().id());
            for error in [&first, &second] {
                assert_eq!(logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source).len(), 1);
                assert!(logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Handled).is_empty());
            }
        });
    }

    #[test]
    fn nested_scene_failure_keeps_original_core_diagnostic_and_popup_context() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture};
        let mut app = scene_app();
        app.add_plugins(WidgetryTooltipPlugin)
            .set_error_handler(ErrorCapture::handler());
        let original = Arc::new(std::sync::Mutex::new(None));
        let sink = original.clone();
        let anchor = app.world_mut().spawn_scene(bsn! {
            @WidgetryTooltip { @content: {TooltipContentFactory::new(move || {
                let sink = sink.clone();
                bsn_list! {template(move |_| {
                    let error = bevy_widgetry_core::color::validate_color(Color::linear_rgba(f32::NAN, 0., 0., 1.)).unwrap_err();
                    *sink.lock().unwrap() = Some(error.origin().clone());
                    Err::<Node,_>(error.into_bevy_error())
                })}
            })} }
        }).unwrap().id();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                crate::headless::trigger_show(app.world_mut(), anchor);
                app.world_mut().flush();
            })
        });
        let errors = errors.take();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = errors[0]
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(Arc::ptr_eq(
            original.lock().unwrap().as_ref().unwrap(),
            diagnostic.origin()
        ));
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        let frame = diagnostic
            .contexts()
            .iter()
            .find(|frame| frame.operation() == "tooltip.show")
            .unwrap();
        assert!(
            frame
                .fields()
                .contains(&("anchor".into(), anchor.to_string()))
        );
        assert!(frame.fields().iter().any(|(name, _)| name == "popup"));
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<TooltipPopup>>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn normal_show_hide_duplicate_show_and_missing_targets_are_diagnostically_silent() {
        let mut app = scene_app();
        app.add_plugins(WidgetryTooltipPlugin);
        let anchor = spawn_tooltip(&mut app, Arc::new(AtomicUsize::new(0)));
        let logs = LogCapture::default();
        logs.run(|| {
            for _ in 0..2 {
                crate::headless::trigger_show(app.world_mut(), anchor);
                app.world_mut().flush();
            }
            crate::headless::trigger_hide(app.world_mut(), anchor);
            crate::headless::trigger_show(app.world_mut(), Entity::PLACEHOLDER);
            crate::headless::trigger_hide(app.world_mut(), Entity::PLACEHOLDER);
            app.world_mut().flush();
        });
        assert!(logs.records().iter().all(|record| record.phase().is_none()));
    }

    #[test]
    fn both_popup_style_failures_reach_the_handler_and_keep_identity_until_recovery() {
        use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture};
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_plugins(WidgetryTooltipPlugin)
            .set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let a = spawn_tooltip(&mut app, Arc::new(AtomicUsize::new(0)));
        let b = spawn_tooltip(&mut app, Arc::new(AtomicUsize::new(0)));
        for anchor in [a, b] {
            bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                popup_scene(
                    anchor,
                    Box::new(bsn_list! {Node}),
                    bevy_widgetry_theme::WIDGETRY_DARK_THEME.tooltip,
                ),
            )
            .unwrap();
        }
        app.world_mut().flush();
        let popups = app
            .world_mut()
            .query_filtered::<(Entity, &ChildOf), With<TooltipPopup>>()
            .iter(app.world())
            .map(|(entity, parent)| (entity, parent.parent()))
            .collect::<Vec<_>>();
        assert_eq!(popups.len(), 2);
        for anchor in [a, b] {
            app.world_mut()
                .entity_mut(anchor)
                .remove::<crate::colors::ColorState>();
        }
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        logs.run(|| {
            errors.run(|| {
                WidgetryTheme::set_in_world(
                    app.world_mut(),
                    bevy_widgetry_theme::WIDGETRY_LIGHT_THEME,
                )
                .unwrap()
            });
            let failures = errors.take();
            assert_eq!(failures.len(), 1);
            let managed = failures[0]
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .unwrap();
            let origins = managed
                .diagnostics()
                .map(|diagnostic| diagnostic.origin().clone())
                .collect::<Vec<_>>();
            assert_eq!(origins.len(), 2);
            assert_ne!(origins[0].id(), origins[1].id());
            for diagnostic in managed.diagnostics() {
                let kind = diagnostic
                    .source()
                    .unwrap()
                    .source()
                    .unwrap()
                    .downcast_ref::<TooltipError>()
                    .unwrap();
                let TooltipError::AnchorColors {
                    anchor,
                    popup: Some(popup),
                    source: Some(_),
                } = kind
                else {
                    panic!("{kind}");
                };
                assert!(popups.contains(&(*popup, *anchor)));
            }
            errors.run(|| app.update());
            let repeated = errors.take();
            assert_eq!(repeated.len(), 1);
            let managed = repeated[0]
                .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
                .unwrap();
            for diagnostic in managed.diagnostics() {
                assert!(
                    origins
                        .iter()
                        .any(|origin| Arc::ptr_eq(origin, diagnostic.origin()))
                );
            }
            for origin in &origins {
                assert_eq!(
                    logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                        .len(),
                    1
                );
                assert!(
                    logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                        .is_empty()
                );
            }
            for anchor in [a, b] {
                app.world_mut()
                    .entity_mut(anchor)
                    .insert(crate::colors::ColorState::default());
            }
            errors.run(|| app.update());
            assert!(errors.take().is_empty());
            for origin in &origins {
                assert_eq!(
                    logs.records_for_error(origin.id())
                        .iter()
                        .filter(|record| record.level == bevy::log::Level::INFO)
                        .count(),
                    1
                );
            }
        });
    }

    fn spawn_tooltip(app: &mut App, calls: Arc<AtomicUsize>) -> Entity {
        app.world_mut()
            .spawn_scene(bsn! {
                @WidgetryTooltip { @content: {TooltipContentFactory::new(move || {
                    calls.fetch_add(1, Ordering::Relaxed);
                    bsn_list!{ContentMarker-- Node Children [Text("details") bevy_widgetry_core::text::WidgetryText]}
                })} }
            })
            .unwrap()
            .id()
    }

    fn popup(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<TooltipPopup>>()
            .single(app.world())
            .unwrap()
    }

    #[test]
    fn colors_stable_popup_skips_resolution_and_tracks_anchor_rebinding() {
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_plugins(WidgetryTooltipPlugin);
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
        });
        let calls = Arc::new(AtomicUsize::new(0));
        let a = spawn_tooltip(&mut app, calls.clone());
        let b = spawn_tooltip(&mut app, calls.clone());
        crate::headless::trigger_show(app.world_mut(), a);
        app.world_mut().flush();
        let popup = popup(&mut app);
        app.update();
        STYLE_WORK.set(0);
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(STYLE_WORK.get(), 0);
        let mut colors = crate::WidgetryTooltipColorOverrides::default();
        colors.popup.normal.foreground = Some(Color::srgb(0.8, 0.1, 0.2));
        crate::WidgetryTooltipColorOverrides::set_in_world(app.world_mut(), b, colors).unwrap();
        app.update();
        assert_eq!(STYLE_WORK.get(), 0);
        app.world_mut().entity_mut(popup).insert(ChildOf(b));
        app.update();
        assert_eq!(STYLE_WORK.replace(0), 1);
        assert_eq!(
            app.world().get::<ResolvedForeground>(popup).unwrap().0,
            Color::srgb(0.8, 0.1, 0.2)
        );
        crate::WidgetryTooltipColorOverrides::clear_in_world(app.world_mut(), b).unwrap();
        app.update();
        assert_eq!(STYLE_WORK.replace(0), 1);
        assert_eq!(
            app.world().get::<ResolvedForeground>(popup).unwrap().0,
            bevy_widgetry_theme::WIDGETRY_DARK_THEME
                .tooltip
                .popup
                .normal
                .foreground
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn colors_first_popup_uses_anchor_override_before_style_schedule() {
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_plugins(WidgetryTooltipPlugin);
        let anchor = spawn_tooltip(&mut app, Arc::new(AtomicUsize::new(0)));
        let mut colors = crate::WidgetryTooltipColorOverrides::default();
        colors.popup.normal.background = Some(Color::srgb(0.2, 0.4, 0.6));
        colors.popup.normal.border = Some(Color::srgb(0.6, 0.4, 0.2));
        colors.popup.normal.foreground = Some(Color::srgb(0.8, 0.6, 0.4));
        crate::WidgetryTooltipColorOverrides::set_in_world(app.world_mut(), anchor, colors.clone())
            .unwrap();
        crate::headless::trigger_show(app.world_mut(), anchor);
        app.world_mut().flush();
        let popup = popup(&mut app);
        assert_eq!(
            Some(app.world().get::<BackgroundColor>(popup).unwrap().0),
            colors.popup.normal.background
        );
        assert_eq!(
            Some(app.world().get::<BorderColor>(popup).unwrap().top),
            colors.popup.normal.border
        );
        assert_eq!(
            Some(app.world().get::<ResolvedForeground>(popup).unwrap().0),
            colors.popup.normal.foreground
        );
    }

    #[test]
    fn missing_content_returns_error_and_logs() {
        let capture = LogCapture::default();
        let result = capture.run(|| {
            let mut app = scene_app();
            app.add_plugins(WidgetryTooltipPlugin);
            app.world_mut()
                .spawn_scene(bsn! { @WidgetryTooltip })
                .map(|entity| entity.id())
        });
        assert!(result.is_err());
        assert_eq!(
            capture
                .records()
                .iter()
                .filter(|record| record.phase()
                    == Some(bevy_widgetry_test_utils::DiagnosticPhase::Source))
                .count(),
            1
        );
    }

    #[test]
    fn failed_popup_content_reaches_handler_without_followup_error() {
        use bevy_widgetry_test_utils::ErrorCapture;
        let mut app = scene_app();
        app.add_plugins(WidgetryTooltipPlugin);
        app.set_error_handler(ErrorCapture::handler());
        let anchor = app.world_mut().spawn_scene(bsn! {
            @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list!{template(|_| Err::<Node, _>(BevyError::error("content failed")))})} }
        }).unwrap().id();
        let errors = ErrorCapture::default();
        errors.run(|| {
            crate::headless::trigger_show(app.world_mut(), anchor);
            app.world_mut().flush();
        });
        let errors = errors.take();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity(), bevy::ecs::error::Severity::Error);
        assert!(app.world().get::<WidgetryTooltip>(anchor).is_some());
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<TooltipPopup>>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn scene_keeps_identity_and_marker_on_anchor_without_popup() {
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_plugins(WidgetryTooltipPlugin);
        let anchor = spawn_tooltip(&mut app, Arc::new(AtomicUsize::new(0)));
        assert!(app.world().get::<WidgetryTooltip>(anchor).is_some());
        assert!(app.world().get::<WidgetryTooltipHeadless>(anchor).is_some());
        assert_eq!(
            app.world_mut()
                .query_filtered::<Entity, With<TooltipPopup>>()
                .iter(app.world())
                .count(),
            0
        );
    }

    #[test]
    fn show_builds_styled_popover_and_arbitrary_content() {
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_message::<bevy::picking::pointer::PointerInput>()
            .add_plugins(WidgetryTooltipPlugin);
        let calls = Arc::new(AtomicUsize::new(0));
        let anchor = spawn_tooltip(&mut app, calls.clone());
        crate::headless::trigger_show(app.world_mut(), anchor);
        app.world_mut().flush();
        let popup = popup(&mut app);
        assert_eq!(app.world().get::<ChildOf>(popup).unwrap().parent(), anchor);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(app.world().get::<Children>(popup).unwrap().len(), 2);
        assert!(
            app.world()
                .get::<ContentMarker>(app.world().get::<Children>(popup).unwrap()[0])
                .is_some()
        );
        assert_eq!(
            app.world().get::<GlobalZIndex>(popup).unwrap().0,
            z_index::TOOLTIP
        );
        assert_eq!(
            *app.world().get::<Pickable>(popup).unwrap(),
            Pickable::IGNORE
        );
        assert!(app.world().get::<OverrideClip>(popup).is_some());
        let node = app.world().get::<Node>(popup).unwrap();
        assert_eq!(node.position_type, PositionType::Absolute);
        assert_eq!(node.padding, UiRect::axes(px(8), px(6)));
        assert_eq!(node.border, UiRect::all(px(1)));
        assert_eq!(node.border_radius, BorderRadius::all(px(4)));
        let popover = app.world().get::<Popover>(popup).unwrap();
        assert_eq!(
            popover.positions,
            vec![
                PopoverPlacement {
                    side: PopoverSide::Bottom,
                    align: PopoverAlign::Center,
                    gap: 6.0
                },
                PopoverPlacement {
                    side: PopoverSide::Top,
                    align: PopoverAlign::Center,
                    gap: 6.0
                },
                PopoverPlacement {
                    side: PopoverSide::Right,
                    align: PopoverAlign::Center,
                    gap: 6.0
                },
                PopoverPlacement {
                    side: PopoverSide::Left,
                    align: PopoverAlign::Center,
                    gap: 6.0
                },
            ]
        );
        assert_eq!(popover.window_margin, 8.0);
        assert_eq!(
            app.world().get::<BackgroundColor>(popup).unwrap().0,
            bevy_widgetry_theme::WIDGETRY_DARK_THEME
                .tooltip
                .popup
                .normal
                .background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(popup).unwrap(),
            BorderColor::all(
                bevy_widgetry_theme::WIDGETRY_DARK_THEME
                    .tooltip
                    .popup
                    .normal
                    .border
            )
        );
        assert_eq!(
            app.world().get::<ResolvedForeground>(popup).unwrap().0,
            bevy_widgetry_theme::WIDGETRY_DARK_THEME
                .tooltip
                .popup
                .normal
                .foreground
        );
        app.update();
        let mut descendants = app.world().get::<Children>(popup).unwrap().to_vec();
        let mut index = 0;
        while index < descendants.len() {
            if let Some(children) = app.world().get::<Children>(descendants[index]) {
                descendants.extend(children.iter());
            }
            index += 1;
        }
        for descendant in descendants {
            assert_eq!(
                app.world().get::<Pickable>(descendant),
                Some(&Pickable::IGNORE)
            );
        }
    }

    #[test]
    fn repeated_show_hide_rebuilds_content() {
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_plugins(WidgetryTooltipPlugin);
        let calls = Arc::new(AtomicUsize::new(0));
        let anchor = spawn_tooltip(&mut app, calls.clone());
        crate::headless::trigger_show(app.world_mut(), anchor);
        app.world_mut().flush();
        let first_popup = popup(&mut app);
        let first_content = app.world().get::<Children>(first_popup).unwrap()[0];
        crate::headless::trigger_hide(app.world_mut(), anchor);
        app.world_mut().flush();
        assert!(app.world().get_entity(first_popup).is_err());
        assert!(app.world().get_entity(first_content).is_err());
        crate::headless::trigger_show(app.world_mut(), anchor);
        app.world_mut().flush();
        assert_ne!(popup(&mut app), first_popup);
        assert_eq!(calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn anchor_despawn_removes_popup_hierarchy() {
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_plugins(WidgetryTooltipPlugin);
        let anchor = spawn_tooltip(&mut app, Arc::new(AtomicUsize::new(0)));
        crate::headless::trigger_show(app.world_mut(), anchor);
        app.world_mut().flush();
        let popup = popup(&mut app);
        let content = app.world().get::<Children>(popup).unwrap()[0];
        app.world_mut().entity_mut(anchor).despawn();
        app.world_mut().flush();
        assert!(app.world().get_entity(popup).is_err());
        assert!(app.world().get_entity(content).is_err());
    }

    #[test]
    fn theme_change_refreshes_visible_popup() {
        let mut app = scene_app();
        app.init_resource::<bevy::picking::hover::HoverMap>()
            .add_plugins(WidgetryTooltipPlugin);
        let anchor = spawn_tooltip(&mut app, Arc::new(AtomicUsize::new(0)));
        crate::headless::trigger_show(app.world_mut(), anchor);
        app.world_mut().flush();
        let popup = popup(&mut app);
        WidgetryTheme::set_in_world(app.world_mut(), bevy_widgetry_theme::WIDGETRY_LIGHT_THEME)
            .expect("theme switch succeeds");
        assert_eq!(
            app.world().get::<BackgroundColor>(popup).unwrap().0,
            bevy_widgetry_theme::WIDGETRY_LIGHT_THEME
                .tooltip
                .popup
                .normal
                .background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(popup).unwrap(),
            BorderColor::all(
                bevy_widgetry_theme::WIDGETRY_LIGHT_THEME
                    .tooltip
                    .popup
                    .normal
                    .border
            )
        );
        assert_eq!(
            app.world().get::<ResolvedForeground>(popup).unwrap().0,
            bevy_widgetry_theme::WIDGETRY_LIGHT_THEME
                .tooltip
                .popup
                .normal
                .foreground
        );
    }
}
