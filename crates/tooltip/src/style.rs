use crate::headless::{HideTooltip, ShowTooltip, Tooltip, TooltipPlugin};
use bevy::{
    app::Propagate,
    picking::PickingSystems,
    prelude::*,
    ui::OverrideClip,
    ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverPlugin, PopoverSide},
};
use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
use bevy_widgetry_core::{
    ForegroundColor, ForegroundColorPlugin, ThemeChanged, ThemeMode, ThemePlugin, z_index,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::sync::Arc;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryTooltipProps)]
pub struct WidgetryTooltip;

pub struct WidgetryTooltipProps {
    pub content: TooltipContentFactory,
}

#[derive(Clone)]
pub struct TooltipContentFactory(Option<Arc<dyn Fn() -> Box<dyn SceneList> + Send + Sync>>);

#[derive(Component)]
struct TooltipContent(TooltipContentFactory);

#[derive(Component, Default, Clone)]
pub(crate) struct TooltipPopup;

pub struct WidgetryTooltipPlugin;

impl TooltipContentFactory {
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn() -> S + Send + Sync + 'static,
    {
        Self(Some(Arc::new(move || Box::new(factory()))))
    }

    fn build(&self) -> Result<Box<dyn SceneList>, BevyError> {
        let Some(factory) = &self.0 else {
            widgetry_error!("Tooltip content factory 缺失");
            return Err(BevyError::error("WidgetryTooltip requires content"));
        };
        Ok(factory())
    }
}

impl Default for WidgetryTooltipProps {
    fn default() -> Self {
        Self {
            content: TooltipContentFactory(None),
        }
    }
}

impl WidgetryTooltip {
    fn scene(props: WidgetryTooltipProps) -> impl Scene {
        let missing_content = props.content.0.is_none();
        bsn! {
            template(move |_| {
                if missing_content {
                    widgetry_error!("Tooltip 构造必须提供 content");
                    return Err(bevy_widgetry_core::scene::logged_error("WidgetryTooltip requires content"));
                }
                Ok(Tooltip)
            })
            template(move |_| Ok(TooltipContent(props.content.clone())))
        }
    }
}

fn popup_scene(anchor: Entity, content: Box<dyn SceneList>) -> impl Scene {
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
        template(|context| Ok(BackgroundColor(context.resource::<ThemeMode>().colors().popup_background)))
        template(|context| Ok(BorderColor::all(context.resource::<ThemeMode>().colors().popup_border)))
        template(|context| Ok(Propagate(ForegroundColor(context.resource::<ThemeMode>().colors().foreground))))
        Node {
            position_type: PositionType::Absolute,
            padding: UiRect::axes(px(8), px(6)),
            border: UiRect::all(px(1)),
            border_radius: BorderRadius::all(px(4)),
        }
        Children [{content}]
    }
}

fn show_tooltip(
    event: On<ShowTooltip>,
    anchors: Query<(&TooltipContent, Option<&Children>), With<WidgetryTooltip>>,
    popups: Query<(), With<TooltipPopup>>,
    mut commands: Commands,
) -> Result<(), BevyError> {
    let Ok((content, children)) = anchors.get(event.source) else {
        return Ok(());
    };
    if children.is_some_and(|children| children.iter().any(|child| popups.contains(child))) {
        return Ok(());
    }
    commands.spawn_scene_with_error_handler(popup_scene(event.source, content.0.build()?));
    Ok(())
}

fn hide_tooltip(
    event: On<HideTooltip>,
    anchors: Query<&Children>,
    popups: Query<(), With<TooltipPopup>>,
    mut commands: Commands,
) {
    let Ok(children) = anchors.get(event.source) else {
        return;
    };
    for child in children.iter() {
        if popups.contains(child) {
            commands.entity(child).despawn();
        }
    }
}

fn refresh_tooltip_theme(
    event: On<ThemeChanged>,
    mut popups: Query<
        (
            &mut BackgroundColor,
            &mut BorderColor,
            &mut Propagate<ForegroundColor>,
        ),
        With<TooltipPopup>,
    >,
) {
    for (mut background, mut border, mut foreground) in &mut popups {
        background.0 = event.mode.colors().popup_background;
        *border = BorderColor::all(event.mode.colors().popup_border);
        foreground.0 = ForegroundColor(event.mode.colors().foreground);
    }
}

fn ignore_tooltip_descendants(
    added: Query<(Entity, &ChildOf), Added<ChildOf>>,
    parents: Query<&ChildOf>,
    popups: Query<(), With<TooltipPopup>>,
    mut commands: Commands,
) {
    for (entity, parent) in &added {
        if popups.contains(parent.parent())
            || parents
                .iter_ancestors(parent.parent())
                .any(|ancestor| popups.contains(ancestor))
        {
            commands.entity(entity).insert(Pickable::IGNORE);
        }
    }
}

impl Plugin for WidgetryTooltipPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<TooltipPlugin>() {
            app.add_plugins(TooltipPlugin);
        }
        if !app.is_plugin_added::<PopoverPlugin>() {
            app.add_plugins(PopoverPlugin);
        }
        if !app.is_plugin_added::<ForegroundColorPlugin>() {
            app.add_plugins(ForegroundColorPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(show_tooltip)
            .add_observer(hide_tooltip)
            .add_observer(refresh_tooltip_theme)
            .add_systems(
                PreUpdate,
                ignore_tooltip_descendants.before(PickingSystems::Backend),
            );
        widgetry_info!("WidgetryTooltipPlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME};
    use bevy_widgetry_test_utils::{LogCapture, scene_app};
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Component, Default, Clone)]
    struct ContentMarker;

    fn spawn_tooltip(app: &mut App, calls: Arc<AtomicUsize>) -> Entity {
        app.world_mut()
            .spawn_scene(bsn! {
                @WidgetryTooltip { @content: {TooltipContentFactory::new(move || {
                    calls.fetch_add(1, Ordering::Relaxed);
                    bsn_list![ContentMarker, (Node Children [Text("details")])]
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
        assert!(capture.records().iter().any(|record| {
            record.level == bevy::log::Level::ERROR
                && record
                    .fields
                    .get("message")
                    .is_some_and(|message| message.contains("Tooltip"))
        }));
    }

    #[test]
    fn failed_popup_content_reaches_handler_without_followup_error() {
        use bevy_widgetry_test_utils::ErrorCapture;
        let mut app = scene_app();
        app.add_plugins(WidgetryTooltipPlugin);
        app.set_error_handler(ErrorCapture::handler());
        let anchor = app.world_mut().spawn_scene(bsn! {
            @WidgetryTooltip { @content: {TooltipContentFactory::new(|| bsn_list![(template(|_| Err::<Node, _>(BevyError::error("content failed"))))])} }
        }).unwrap().id();
        let errors = ErrorCapture::default();
        errors.run(|| {
            app.world_mut().trigger(ShowTooltip { source: anchor });
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
        assert!(app.world().get::<Tooltip>(anchor).is_some());
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
        app.world_mut().trigger(ShowTooltip { source: anchor });
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
            DARK_THEME.popup_background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(popup).unwrap(),
            BorderColor::all(DARK_THEME.popup_border)
        );
        assert_eq!(
            app.world()
                .get::<Propagate<ForegroundColor>>(popup)
                .unwrap()
                .0,
            ForegroundColor(DARK_THEME.foreground)
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
        app.world_mut().trigger(ShowTooltip { source: anchor });
        app.world_mut().flush();
        let first_popup = popup(&mut app);
        let first_content = app.world().get::<Children>(first_popup).unwrap()[0];
        app.world_mut().trigger(HideTooltip { source: anchor });
        app.world_mut().flush();
        assert!(app.world().get_entity(first_popup).is_err());
        assert!(app.world().get_entity(first_content).is_err());
        app.world_mut().trigger(ShowTooltip { source: anchor });
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
        app.world_mut().trigger(ShowTooltip { source: anchor });
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
        app.world_mut().trigger(ShowTooltip { source: anchor });
        app.world_mut().flush();
        let popup = popup(&mut app);
        app.world_mut().trigger(ThemeChanged {
            mode: bevy_widgetry_core::ThemeMode::Light,
        });
        assert_eq!(
            app.world().get::<BackgroundColor>(popup).unwrap().0,
            LIGHT_THEME.popup_background
        );
        assert_eq!(
            *app.world().get::<BorderColor>(popup).unwrap(),
            BorderColor::all(LIGHT_THEME.popup_border)
        );
        assert_eq!(
            app.world()
                .get::<Propagate<ForegroundColor>>(popup)
                .unwrap()
                .0,
            ForegroundColor(LIGHT_THEME.foreground)
        );
    }
}
