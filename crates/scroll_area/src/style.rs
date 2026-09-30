use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{GridPlacement, RepeatedGridTrack, ScrollPosition};
use bevy::ui_widgets::{
    ControlOrientation, ScrollArea, Scrollbar, ScrollbarDragState, ScrollbarThumb,
};
use bevy_widgetry_core::{ColorTheme, ThemeChanged, ThemeMode};

use crate::headless::{
    ScrollAxis, ScrollbarVisibility, WidgetryScrollAreaContent, WidgetryScrollAreaViewport,
};
use crate::layout::{
    DEFAULT_SCROLLBAR_THICKNESS, HorizontalScrollbar, ScrollAreaConfig, VerticalScrollbar,
};

const DEFAULT_MIN_THUMB_LENGTH: f32 = 24.0;

/// 只给 Widgetry 自建 Thumb 应用 theme，避免影响同一 App 的其他 Scrollbar。
#[derive(Component, Default, Clone)]
pub(crate) struct ScrollAreaThumb;

/// 公开的 ScrollArea identity；通过 @WidgetryScrollArea 构造完整 hierarchy。
/// 应用需注册 WidgetryScrollAreaPlugin，调用方可在其后 patch root Node。
#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryScrollAreaProps)]
pub struct WidgetryScrollArea;

/// 只在 Scene 构造期间消费的 ScrollArea 配置与内容。
pub struct WidgetryScrollAreaProps {
    /// 构造后固定的滚动轴。
    pub axis: ScrollAxis,
    /// 各轴 scrollbar 的显示策略。
    pub scrollbar_visibility: ScrollbarVisibility,
    /// Scrollbar track 的横向或纵向厚度，单位为 logical px。
    pub scrollbar_thickness: f32,
    /// 构造后固定的 keyboard-scroll 开关，默认 true；关闭不影响 wheel 或程序化滚动。
    pub keyboard_scroll: bool,
    /// 应用于内部 Content entity 的一次性 Scene patch。
    pub content: Option<Box<dyn Scene>>,
    /// 放入 Content entity 的一次性 children。
    pub children: Option<Box<dyn SceneList>>,
}

impl Default for WidgetryScrollAreaProps {
    fn default() -> Self {
        Self {
            axis: ScrollAxis::default(),
            scrollbar_visibility: ScrollbarVisibility::default(),
            scrollbar_thickness: DEFAULT_SCROLLBAR_THICKNESS,
            keyboard_scroll: true,
            content: None,
            children: None,
        }
    }
}

impl WidgetryScrollArea {
    /// 让用户 Content patch 先于内部尺寸约束，Scrollbar 引用同一个 Viewport。
    fn scene(props: WidgetryScrollAreaProps) -> impl Scene {
        let WidgetryScrollAreaProps {
            axis,
            scrollbar_visibility,
            scrollbar_thickness,
            keyboard_scroll,
            content,
            children,
        } = props;
        let direction = if axis == ScrollAxis::Horizontal {
            FlexDirection::Row
        } else {
            FlexDirection::Column
        };
        let horizontal = matches!(axis, ScrollAxis::Horizontal | ScrollAxis::Both).then(|| bsn! {
            HorizontalScrollbar
            Scrollbar { target: Entity::PLACEHOLDER, orientation: ControlOrientation::Horizontal, min_thumb_length: DEFAULT_MIN_THUMB_LENGTH }
            Node { grid_column: GridPlacement::start(1), grid_row: GridPlacement::start(2), height: px(scrollbar_thickness) }
            Children [(ScrollAreaThumb ScrollbarThumb { border_radius: BorderRadius::all(px(6)) } Hovered(false))]
        });
        let vertical = matches!(axis, ScrollAxis::Vertical | ScrollAxis::Both).then(|| bsn! {
            VerticalScrollbar
            Scrollbar { target: Entity::PLACEHOLDER, orientation: ControlOrientation::Vertical, min_thumb_length: DEFAULT_MIN_THUMB_LENGTH }
            Node { grid_column: GridPlacement::start(2), grid_row: GridPlacement::start_span(1, 2), width: px(scrollbar_thickness) }
            Children [(ScrollAreaThumb ScrollbarThumb { border_radius: BorderRadius::all(px(6)) } Hovered(false))]
        });
        bsn! {
            TabIndex(-1)
            template(move |_| Ok(ScrollAreaConfig { axis, scrollbar_visibility, scrollbar_thickness, keyboard_scroll }))
            Node {
                display: Display::Grid,
                grid_template_columns: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
                grid_template_rows: vec![RepeatedGridTrack::flex(1, 1.0), RepeatedGridTrack::auto(1)],
            }
            Children [
                (
                    WidgetryScrollAreaViewport
                    ScrollArea
                    ScrollPosition::default()
                    Node { grid_column: GridPlacement::start(1), grid_row: GridPlacement::start(1), overflow: {axis.overflow()}, scrollbar_width: 0.0, align_items: AlignItems::FlexStart }
                    Children [(
                        WidgetryScrollAreaContent
                        Node { flex_direction: direction }
                        {content}
                        Node { width: Val::Auto, height: Val::Auto, min_width: percent(100), min_height: percent(100), max_width: Val::Auto, max_height: Val::Auto, flex_shrink: 0.0, overflow: Overflow::visible() }
                        Children [{children}]
                    )]
                ),
                {horizontal},
                {vertical},
            ]
        }
    }
}

/// 根据官方 drag 与 hover state 更新 Thumb 背景，不干涉官方几何计算。
fn apply_thumb_style(
    colors: &ColorTheme,
    hovered: &Hovered,
    drag: &ScrollbarDragState,
    background: &mut BackgroundColor,
) {
    background.0 = if drag.dragging {
        colors.control_border_pressed
    } else if hovered.0 {
        colors.control_border_hovered
    } else {
        colors.control_border
    };
}

/// Thumb 新增或交互 state 改变时解析当前 theme。
pub(crate) fn update_thumb_style(
    mode: Res<ThemeMode>,
    mut thumbs: Query<
        (&Hovered, &ScrollbarDragState, &mut BackgroundColor),
        (
            With<ScrollbarThumb>,
            With<ScrollAreaThumb>,
            Or<(
                Added<ScrollbarThumb>,
                Changed<Hovered>,
                Changed<ScrollbarDragState>,
            )>,
        ),
    >,
) {
    for (hovered, drag, mut background) in &mut thumbs {
        apply_thumb_style(mode.colors(), hovered, drag, &mut background);
    }
}

/// ThemeChanged 时按现有 interaction state 立即刷新所有 Thumb。
pub(crate) fn refresh_theme(
    event: On<ThemeChanged>,
    mut thumbs: Query<
        (&Hovered, &ScrollbarDragState, &mut BackgroundColor),
        (With<ScrollbarThumb>, With<ScrollAreaThumb>),
    >,
) {
    for (hovered, drag, mut background) in &mut thumbs {
        apply_thumb_style(event.mode.colors(), hovered, drag, &mut background);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::camera::{Camera2d, ComputedCameraValues, RenderTargetInfo, Viewport};
    use bevy::text::{FontCx, ScaleCx, TextPipeline};
    use bevy::ui::{ComputedNode, OverflowAxis, UiPlugin};
    use bevy_widgetry_test_utils::scene_app;

    fn parts(world: &World, root: Entity) -> (Entity, Entity, Vec<Entity>) {
        let children = world.get::<Children>(root).unwrap();
        let viewport = children
            .iter()
            .find(|&child| world.get::<WidgetryScrollAreaViewport>(child).is_some())
            .unwrap();
        let content = world.get::<Children>(viewport).unwrap()[0];
        let bars = children
            .iter()
            .filter(|&child| world.get::<Scrollbar>(child).is_some())
            .collect();
        (viewport, content, bars)
    }

    /// 默认 Props 允许空内容，公开 Scene 构造出无固定尺寸的 root、Viewport 和 Content。
    #[test]
    fn default_scene_has_expected_hierarchy() {
        let props = WidgetryScrollAreaProps::default();
        assert_eq!(props.axis, ScrollAxis::Vertical);
        assert_eq!(props.scrollbar_visibility, ScrollbarVisibility::default());
        assert_eq!(props.scrollbar_thickness, 12.0);
        assert!(props.content.is_none() && props.children.is_none());
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryScrollArea })
            .unwrap()
            .id();
        let (viewport, content, bars) = parts(app.world(), root);
        app.update();
        assert_eq!(bars.len(), 1);
        assert_eq!(
            app.world().get::<Scrollbar>(bars[0]).unwrap().target,
            viewport
        );
        assert_eq!(
            app.world().get::<Scrollbar>(bars[0]).unwrap().orientation,
            ControlOrientation::Vertical
        );
        assert_eq!(
            app.world()
                .get::<Scrollbar>(bars[0])
                .unwrap()
                .min_thumb_length,
            24.0
        );
        assert!(app.world().get::<ScrollPosition>(viewport).is_some());
        assert_eq!(app.world().get::<Children>(viewport).unwrap().len(), 1);
        assert_eq!(app.world().get::<Children>(content).unwrap().len(), 0);
        let root_node = app.world().get::<Node>(root).unwrap();
        assert_eq!(root_node.display, Display::Grid);
        assert_eq!(root_node.width, Val::Auto);
        assert_eq!(root_node.height, Val::Auto);
        assert_eq!(app.world().get::<TabIndex>(root).unwrap().0, -1);
        let viewport_node = app.world().get::<Node>(viewport).unwrap();
        assert_eq!(viewport_node.grid_column, GridPlacement::start(1));
        assert_eq!(viewport_node.grid_row, GridPlacement::start(1));
        assert_eq!(viewport_node.scrollbar_width, 0.0);
        assert_eq!(viewport_node.overflow.x, OverflowAxis::Clip);
        assert_eq!(viewport_node.overflow.y, OverflowAxis::Scroll);
        assert_eq!(
            app.world().get::<Node>(content).unwrap().flex_direction,
            FlexDirection::Column
        );
    }

    /// 两轴配置建立两个指向同一 Viewport 的官方 Scrollbar，Content patch 只覆盖允许字段。
    #[test]
    fn both_axes_and_content_patch_keep_sizing_invariant() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app.world_mut().spawn_scene(bsn! {
            @WidgetryScrollArea {
                @axis: ScrollAxis::Both,
                @scrollbar_thickness: 17.0,
                @content: bsn! { Node { flex_direction: FlexDirection::Row, width: px(44), min_width: px(3), padding: UiRect::all(px(5)) } },
                @children: bsn_list![(Node { width: px(200), height: px(300) })],
            }
        }).unwrap().id();
        let (viewport, content, bars) = parts(app.world(), root);
        app.update();
        assert_eq!(bars.len(), 2);
        assert!(
            bars.iter()
                .all(|&bar| app.world().get::<Scrollbar>(bar).unwrap().target == viewport)
        );
        let h = bars
            .iter()
            .copied()
            .find(|&bar| app.world().get::<HorizontalScrollbar>(bar).is_some())
            .unwrap();
        let v = bars
            .iter()
            .copied()
            .find(|&bar| app.world().get::<VerticalScrollbar>(bar).is_some())
            .unwrap();
        assert_eq!(app.world().get::<Node>(h).unwrap().height, px(17));
        assert_eq!(app.world().get::<Node>(v).unwrap().width, px(17));
        assert_eq!(
            app.world().get::<Node>(viewport).unwrap().overflow,
            Overflow::scroll()
        );
        let node = app.world().get::<Node>(content).unwrap();
        assert_eq!(node.flex_direction, FlexDirection::Row);
        assert_eq!(node.padding, UiRect::all(px(5)));
        assert_eq!(node.width, Val::Auto);
        assert_eq!(node.min_width, percent(100));
        assert_eq!(node.min_height, percent(100));
        assert_eq!(node.flex_shrink, 0.0);
        assert_eq!(node.overflow, Overflow::visible());
        assert_eq!(app.world().get::<Children>(content).unwrap().len(), 1);
    }

    /// 交互 state 的优先级及 ThemeChanged 对现有 Thumb 的即时颜色刷新保持一致。
    #[test]
    fn thumb_style_tracks_hover_drag_and_theme() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryScrollArea })
            .unwrap()
            .id();
        let (_, _, bars) = parts(app.world(), root);
        let thumb = app.world().get::<Children>(bars[0]).unwrap()[0];
        assert!(app.world().get::<ScrollbarThumb>(thumb).is_some());
        assert!(app.world().get::<ScrollbarDragState>(thumb).is_some());
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            ThemeMode::Dark.colors().control_border
        );
        app.world_mut().entity_mut(thumb).insert(Hovered(true));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            ThemeMode::Dark.colors().control_border_hovered
        );
        app.world_mut()
            .get_mut::<ScrollbarDragState>(thumb)
            .unwrap()
            .dragging = true;
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            ThemeMode::Dark.colors().control_border_pressed
        );
        *app.world_mut().resource_mut::<ThemeMode>() = ThemeMode::Light;
        app.world_mut().trigger(ThemeChanged {
            mode: ThemeMode::Light,
        });
        assert_eq!(
            app.world().get::<BackgroundColor>(thumb).unwrap().0,
            ThemeMode::Light.colors().control_border_pressed
        );
    }

    /// 注册 Widgetry ScrollAreaPlugin 后，其他官方 ScrollbarThumb 保留自己的颜色。
    #[test]
    fn unrelated_scrollbar_thumb_keeps_its_color() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryScrollAreaPlugin);
        let external = app
            .world_mut()
            .spawn((
                ScrollbarThumb::default(),
                Hovered(true),
                BackgroundColor(Color::srgb_u8(12, 34, 56)),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(external).unwrap().0,
            Color::srgb_u8(12, 34, 56)
        );
        *app.world_mut().resource_mut::<ThemeMode>() = ThemeMode::Light;
        app.world_mut().trigger(ThemeChanged {
            mode: ThemeMode::Light,
        });
        assert_eq!(
            app.world().get::<BackgroundColor>(external).unwrap().0,
            Color::srgb_u8(12, 34, 56)
        );
    }

    /// 真实 UI layout 中，Content 在三个 axis 均先填满 Viewport，再由超尺寸 child 自然撑大。
    #[test]
    fn content_sizing_contract_holds_in_real_layout() {
        for axis in [
            ScrollAxis::Vertical,
            ScrollAxis::Horizontal,
            ScrollAxis::Both,
        ] {
            let mut app = scene_app();
            app.init_resource::<FontCx>()
                .init_resource::<ScaleCx>()
                .init_resource::<TextPipeline>()
                .init_resource::<bevy::input::touch::Touches>()
                .add_message::<bevy::window::WindowEvent>()
                .init_asset::<bevy::image::TextureAtlasLayout>()
                .add_plugins(bevy::input::InputPlugin)
                .add_plugins(bevy::picking::DefaultPickingPlugins)
                .add_plugins(bevy::text::TextPlugin)
                .add_plugins(UiPlugin)
                .add_plugins(crate::WidgetryScrollAreaPlugin);
            app.world_mut().spawn((
                Camera2d,
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: UVec2::splat(200),
                            scale_factor: 1.0,
                        }),
                        ..default()
                    },
                    viewport: Some(Viewport {
                        physical_size: UVec2::splat(200),
                        ..default()
                    }),
                    ..default()
                },
            ));
            let root = app.world_mut().spawn_scene(bsn! {
                @WidgetryScrollArea {
                    @axis: {axis},
                    @children: bsn_list![(Node { width: px(150), height: px(150), flex_shrink: 0.0 })],
                }
                Node { width: px(100), height: px(100) }
            }).unwrap().id();
            let (viewport, content, _) = parts(app.world(), root);
            app.update();
            let viewport_size = app.world().get::<ComputedNode>(viewport).unwrap().size();
            let content_size = app.world().get::<ComputedNode>(content).unwrap().size();
            assert!(
                content_size.x >= viewport_size.x,
                "{axis:?}: {content_size:?} < {viewport_size:?}"
            );
            assert!(
                content_size.y >= viewport_size.y,
                "{axis:?}: {content_size:?} < {viewport_size:?}"
            );
            match axis {
                ScrollAxis::Vertical => assert!(
                    content_size.y >= 150.0,
                    "{axis:?}: {content_size:?} / {viewport_size:?}"
                ),
                ScrollAxis::Horizontal => assert!(
                    content_size.x >= 150.0,
                    "{axis:?}: {content_size:?} / {viewport_size:?}"
                ),
                ScrollAxis::Both => assert!(
                    content_size.x >= 150.0 && content_size.y >= 150.0,
                    "{axis:?}: {content_size:?} / {viewport_size:?}"
                ),
            }
        }
    }
}
