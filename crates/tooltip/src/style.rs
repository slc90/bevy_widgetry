use crate::headless::{HideTooltip, ShowTooltip, Tooltip, TooltipPlugin};
use bevy::{
    app::Propagate,
    picking::PickingSystems,
    prelude::*,
    ui::OverrideClip,
    ui_widgets::popover::{Popover, PopoverAlign, PopoverPlacement, PopoverPlugin, PopoverSide},
};
use bevy_widgetry_core::{
    ForegroundColor, ForegroundColorPlugin, ThemeChanged, ThemeMode, ThemePlugin, z_index,
};
use bevy_widgetry_log::{widgetry_error, widgetry_info};
use std::sync::Arc;

/// 使用 Widgetry popup theme 的 Tooltip anchor identity；通过 BSN 的 @WidgetryTooltip 构造。
/// 同 anchor 的 private state 在每次 show 时重新生成任意 SceneList，需注册 WidgetryTooltipPlugin。
#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryTooltipProps)]
pub struct WidgetryTooltip;

/// WidgetryTooltip 的一次性 BSN 构造输入；content 为必填项。
pub struct WidgetryTooltipProps {
    /// 每次显示时重新构造 popup children 的 factory。
    pub content: TooltipContentFactory,
}

/// 可重复构造 Tooltip 内容的 factory，不包含 popup 外壳。
#[derive(Clone)]
pub struct TooltipContentFactory(Option<Arc<dyn Fn() -> Box<dyn SceneList> + Send + Sync>>);

/// 与公开 identity 同 anchor 保存的唯一运行时 factory state。
#[derive(Component)]
struct TooltipContent(TooltipContentFactory);

/// 标识 styled layer 动态创建的 popup entity。
#[derive(Component, Default, Clone)]
pub(crate) struct TooltipPopup;

/// 注册内部 hover state machine、Popover、theme 与 styled popup lifecycle。
pub struct WidgetryTooltipPlugin;

impl TooltipContentFactory {
    /// 接收可重复调用的任意 SceneList factory；closure 捕获的 owned 数据不能被单次消费。
    pub fn new<S, F>(factory: F) -> Self
    where
        S: SceneList + 'static,
        F: Fn() -> S + Send + Sync + 'static,
    {
        Self(Some(Arc::new(move || Box::new(factory()))))
    }

    /// 为一次 show 构造独立内容。
    fn build(&self) -> Box<dyn SceneList> {
        let Some(factory) = &self.0 else {
            widgetry_error!("Tooltip content factory 缺失");
            panic!("WidgetryTooltip requires content");
        };
        factory()
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
    /// 验证必填 content 后，把公开 identity 与内部 marker 放在同一 anchor。
    fn scene(props: WidgetryTooltipProps) -> impl Scene {
        if props.content.0.is_none() {
            widgetry_error!("Tooltip 构造必须提供 content");
            panic!("WidgetryTooltip requires content");
        }
        bsn! {
            Tooltip
            template(move |_| Ok(TooltipContent(props.content.clone())))
        }
    }
}

/// 构造 Tooltip popup 外壳，并把调用方 SceneList 直接作为 children。
fn popup_scene(content: Box<dyn SceneList>) -> impl Scene {
    bsn! {
        TooltipPopup
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

/// Show event 为 anchor 创建唯一 direct-child popup。
fn show_tooltip(
    event: On<ShowTooltip>,
    anchors: Query<(&TooltipContent, Option<&Children>), With<WidgetryTooltip>>,
    popups: Query<(), With<TooltipPopup>>,
    mut commands: Commands,
) {
    let Ok((content, children)) = anchors.get(event.source) else {
        return;
    };
    if children.is_some_and(|children| children.iter().any(|child| popups.contains(child))) {
        return;
    }
    let popup = commands.spawn_scene(popup_scene(content.0.build())).id();
    commands.entity(event.source).add_child(popup);
}

/// Hide event 销毁 anchor 的全部 direct-child Tooltip popup 及其内容 hierarchy。
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

/// ThemeChanged 时立即刷新当前 popup 的 background、border 与传播 foreground。
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

/// 在 UI picking backend 运行前把 popup 的全部新增 descendant 设为非交互，包含任意调用方内容。
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_widgetry_core::{DARK_THEME, LIGHT_THEME};
    use bevy_widgetry_test_utils::{LogCapture, scene_app};
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::atomic::{AtomicUsize, Ordering},
    };

    /// 用于证明 factory 接受任意多 entity SceneList，且 hide 会递归销毁内容。
    #[derive(Component, Default, Clone)]
    struct ContentMarker;

    /// 构造带有效 content 的 Tooltip anchor。
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

    /// 返回当前唯一 popup，测试 helper 的唯一性断言也覆盖同时只能存在一个实例。
    fn popup(app: &mut App) -> Entity {
        app.world_mut()
            .query_filtered::<Entity, With<TooltipPopup>>()
            .single(app.world())
            .unwrap()
    }

    /// 缺少必填 content 违反构造前置条件，必须先记录库 ERROR 再 panic。
    #[test]
    fn missing_content_logs_before_panicking() {
        let capture = LogCapture::default();
        let result = capture.run(|| {
            catch_unwind(AssertUnwindSafe(|| {
                let mut app = scene_app();
                app.add_plugins(WidgetryTooltipPlugin);
                app.world_mut()
                    .spawn_scene(bsn! { @WidgetryTooltip })
                    .unwrap();
            }))
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

    /// Scene 展开后公开 identity 与内部 marker 位于同一 anchor，初始不创建 popup。
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

    /// show 创建 direct-child popup 与任意内容，并精确应用 Popover、layout、picking 和 theme token。
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

    /// hide 递归销毁 popup content；再次 show 必须重新调用 Fn factory 并创建新 entity。
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

    /// anchor despawn 必须随 hierarchy 自动释放 popup 与任意 content entity。
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

    /// ThemeChanged 立即刷新现存 popup 的三类 theme 输出，不重建内容。
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
