use crate::layout::{ScrollAreaConfig, configure_geometry, solve_visibility};
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseScrollUnit;
use bevy::input_focus::FocusedInput;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, Overflow, OverflowAxis, ScrollPosition, UiGlobalTransform};
use bevy::ui_widgets::{ScrollArea, ScrollAreaPlugin, ScrollbarPlugin};
use bevy_widgetry_core::ThemePlugin;

use crate::style::{refresh_theme, update_thumb_style};

/// 构造 ScrollArea 时选择的滚动轴；不支持运行期切换。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollAxis {
    Horizontal,
    #[default]
    Vertical,
    Both,
}

/// 已存在轴的 scrollbar 显示策略；Auto 仅在实际 overflow 时保留 scrollbar 与 gutter。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScrollbarPolicy {
    #[default]
    Auto,
    Always,
    Hidden,
}

/// 两个轴各自的 scrollbar 策略；不存在的轴对应策略不生效。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollbarVisibility {
    pub horizontal: ScrollbarPolicy,
    pub vertical: ScrollbarPolicy,
}

/// 对 descendant 发出请求，由最近的 Widgetry Viewport 将其左上角滚动到可见区域。
#[derive(Copy, Clone, Debug, PartialEq, EntityEvent)]
#[entity_event(propagate)]
pub struct WidgetryScrollIntoView {
    pub entity: Entity,
}

/// 标记持有原生 ScrollPosition 的 Viewport，供调用方查询和程序化滚动。
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WidgetryScrollAreaViewport;

/// Viewport 的唯一 direct child，承载用户内容及其实际 layout geometry。
/// 组合 Widget 可通过此 marker 定位内容挂载点；不应删除或在 Viewport 下增加第二个 content。
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct WidgetryScrollAreaContent;

/// 装配官方 scroll、scrollbar、navigation、theme 与 Widgetry 行为。
/// 应用需提供官方 InputFocusPlugin 和 InputDispatchPlugin，才能接收真实 focus 和 keyboard 输入。
pub struct WidgetryScrollAreaPlugin;

impl Plugin for WidgetryScrollAreaPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<ScrollAreaPlugin>() {
            app.add_plugins(ScrollAreaPlugin);
        }
        if !app.is_plugin_added::<TabNavigationPlugin>() {
            app.add_plugins(TabNavigationPlugin);
        }
        if !app.is_plugin_added::<ScrollbarPlugin>() {
            app.add_plugins(ScrollbarPlugin);
        }
        if !app.is_plugin_added::<ThemePlugin>() {
            app.add_plugins(ThemePlugin);
        }
        app.add_observer(on_keyboard)
            .add_observer(on_scroll_into_view)
            .add_observer(refresh_theme);
        app.add_message::<bevy::window::RequestRedraw>();
        app.add_systems(
            PostUpdate,
            configure_geometry.before(bevy::ui::UiSystems::Layout),
        );
        app.add_systems(
            PostUpdate,
            solve_visibility.after(bevy::ui::UiSystems::Layout),
        );
        app.add_systems(Update, update_thumb_style);
    }
}

impl ScrollAxis {
    /// 生成 Viewport 的原生 overflow；关闭的轴始终 clip。
    pub(crate) fn overflow(self) -> Overflow {
        match self {
            Self::Horizontal => Overflow {
                x: OverflowAxis::Scroll,
                y: OverflowAxis::Clip,
            },
            Self::Vertical => Overflow {
                x: OverflowAxis::Clip,
                y: OverflowAxis::Scroll,
            },
            Self::Both => Overflow::scroll(),
        }
    }
}

fn scroll_range(computed: &ComputedNode) -> Vec2 {
    (computed.content_size() - computed.size()).max(Vec2::ZERO) * computed.inverse_scale_factor
}

fn keyboard_position(
    axis: ScrollAxis,
    key: KeyCode,
    position: Vec2,
    viewport_size: Vec2,
    max_range: Vec2,
) -> Option<Vec2> {
    let line = MouseScrollUnit::SCROLL_UNIT_CONVERSION_FACTOR;
    let overflow = axis.overflow();
    let mut next = position;
    match key {
        KeyCode::ArrowLeft if overflow.x == OverflowAxis::Scroll => next.x -= line,
        KeyCode::ArrowRight if overflow.x == OverflowAxis::Scroll => next.x += line,
        KeyCode::ArrowUp if overflow.y == OverflowAxis::Scroll => next.y -= line,
        KeyCode::ArrowDown if overflow.y == OverflowAxis::Scroll => next.y += line,
        KeyCode::PageUp if overflow.y == OverflowAxis::Scroll => next.y -= viewport_size.y,
        KeyCode::PageDown if overflow.y == OverflowAxis::Scroll => next.y += viewport_size.y,
        KeyCode::Home if overflow.y == OverflowAxis::Scroll => next.y = 0.0,
        KeyCode::End if overflow.y == OverflowAxis::Scroll => next.y = max_range.y,
        _ => return None,
    }
    Some(next.clamp(Vec2::ZERO, max_range))
}

fn on_keyboard(
    mut event: On<FocusedInput<KeyboardInput>>,
    roots: Query<(&ScrollAreaConfig, &Children)>,
    mut viewports: Query<(&ComputedNode, &mut ScrollPosition), With<WidgetryScrollAreaViewport>>,
) {
    if event.event().input.state != ButtonState::Pressed {
        return;
    }
    let Ok((root, children)) = roots.get(event.focused_entity) else {
        return;
    };
    if !root.keyboard_scroll {
        return;
    }
    let Some(viewport) = children.iter().find(|&child| viewports.contains(child)) else {
        return;
    };
    let Ok((computed, mut position)) = viewports.get_mut(viewport) else {
        return;
    };
    let viewport_size = computed.size() * computed.inverse_scale_factor;
    let max_range = scroll_range(computed);
    if let Some(next) = keyboard_position(
        root.axis,
        event.event().input.key_code,
        position.0,
        viewport_size,
        max_range,
    ) {
        event.propagate(false);
        if next != position.0 {
            position.0 = next;
        }
    }
}

fn align_if_outside(
    position: f32,
    viewport_size: f32,
    target_start: f32,
    target_size: f32,
    max: f32,
) -> f32 {
    if target_start < position || target_start + target_size > position + viewport_size {
        target_start.clamp(0.0, max)
    } else {
        position.clamp(0.0, max)
    }
}

fn on_scroll_into_view(
    mut event: On<WidgetryScrollIntoView>,
    parents: Query<&ChildOf>,
    content: Query<(), With<WidgetryScrollAreaContent>>,
    nodes: Query<(&Node, &ComputedNode, &UiGlobalTransform)>,
    mut viewports: Query<&mut ScrollPosition, (With<WidgetryScrollAreaViewport>, With<ScrollArea>)>,
) {
    let target = event.entity;
    let Some(viewport) = parents
        .iter_ancestors(target)
        .find(|&entity| viewports.contains(entity))
    else {
        return;
    };
    let Some(branch) = parents
        .iter_ancestors(target)
        .take_while(|&entity| entity != viewport)
        .last()
    else {
        return;
    };
    if !content.contains(branch) {
        return;
    }
    let (Ok((_, target_computed, target_transform)), Ok((node, computed, transform))) =
        (nodes.get(target), nodes.get(viewport))
    else {
        return;
    };
    let Ok(mut position) = viewports.get_mut(viewport) else {
        return;
    };
    event.propagate(false);
    let viewport_size = computed.size() * computed.inverse_scale_factor;
    let target_size = target_computed.size() * target_computed.inverse_scale_factor;
    let viewport_top_left =
        transform.affine().translation * computed.inverse_scale_factor - viewport_size * 0.5;
    let target_top_left = target_transform.affine().translation
        * target_computed.inverse_scale_factor
        - target_size * 0.5;
    let target_start = target_top_left - viewport_top_left + position.0;
    let max_range = scroll_range(computed);
    let mut next = position.0;
    if node.overflow.x == OverflowAxis::Scroll {
        next.x = align_if_outside(
            next.x,
            viewport_size.x,
            target_start.x,
            target_size.x,
            max_range.x,
        );
    }
    if node.overflow.y == OverflowAxis::Scroll {
        next.y = align_if_outside(
            next.y,
            viewport_size.y,
            target_start.y,
            target_size.y,
            max_range.y,
        );
    }
    if next != position.0 {
        position.0 = next;
    }
}

// 测试 module 中的断言用于验证 contract，生产代码仍禁止。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::input::keyboard::Key;
    use bevy::input_focus::tab_navigation::TabIndex;
    use bevy::input_focus::{InputFocus, InputFocusSystems, dispatch_focused_input};
    use bevy::window::PrimaryWindow;
    use bevy_widgetry_test_utils::{primary_press, scene_app};

    /// 三种 construction-time axis 必须只允许所选方向滚动，另一轴始终 clip。
    #[test]
    fn axis_maps_to_clipped_viewport_overflow() {
        assert_eq!(ScrollAxis::Vertical.overflow().x, OverflowAxis::Clip);
        assert_eq!(ScrollAxis::Vertical.overflow().y, OverflowAxis::Scroll);
        assert_eq!(ScrollAxis::Horizontal.overflow().x, OverflowAxis::Scroll);
        assert_eq!(ScrollAxis::Horizontal.overflow().y, OverflowAxis::Clip);
        assert_eq!(ScrollAxis::Both.overflow().x, OverflowAxis::Scroll);
        assert_eq!(ScrollAxis::Both.overflow().y, OverflowAxis::Scroll);
    }

    /// 同一初始滚动位置下，方向键、翻页键和边界键按 axis 与当前范围计算并 clamp。
    #[test]
    fn keyboard_uses_axis_step_page_and_clamp() {
        let range = Vec2::new(300.0, 500.0);
        let size = Vec2::new(100.0, 120.0);
        let start = Vec2::new(150.0, 200.0);
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::ArrowLeft, start, size, range),
            Some(Vec2::new(50.0, 200.0))
        );
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::ArrowRight, start, size, range),
            Some(Vec2::new(250.0, 200.0))
        );
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::ArrowUp, start, size, range),
            Some(Vec2::new(150.0, 100.0))
        );
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::ArrowDown, start, size, range),
            Some(Vec2::new(150.0, 300.0))
        );
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::PageUp, start, size, range),
            Some(Vec2::new(150.0, 80.0))
        );
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::PageDown, start, size, range),
            Some(Vec2::new(150.0, 320.0))
        );
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::Home, start, size, range),
            Some(Vec2::new(150.0, 0.0))
        );
        assert_eq!(
            keyboard_position(ScrollAxis::Both, KeyCode::End, start, size, range),
            Some(Vec2::new(150.0, 500.0))
        );
        assert_eq!(
            keyboard_position(
                ScrollAxis::Horizontal,
                KeyCode::PageDown,
                start,
                size,
                range
            ),
            None
        );
        assert_eq!(
            keyboard_position(
                ScrollAxis::Horizontal,
                KeyCode::ArrowDown,
                start,
                size,
                range
            ),
            None
        );
        assert_eq!(
            keyboard_position(
                ScrollAxis::Vertical,
                KeyCode::ArrowRight,
                start,
                size,
                range
            ),
            None
        );
        assert_eq!(
            keyboard_position(
                ScrollAxis::Vertical,
                KeyCode::ArrowDown,
                Vec2::new(0.0, 490.0),
                size,
                range
            ),
            Some(Vec2::new(0.0, 500.0))
        );
        assert_eq!(
            keyboard_position(
                ScrollAxis::Vertical,
                KeyCode::ArrowUp,
                Vec2::ZERO,
                size,
                range
            ),
            Some(Vec2::ZERO)
        );
    }

    /// 目标从完全可见变为部分可见、不可见或大于 Viewport 时，按左上角对齐并 clamp。
    #[test]
    fn visibility_aligns_partial_invisible_and_oversized_targets() {
        assert_eq!(align_if_outside(100.0, 100.0, 120.0, 20.0, 300.0), 100.0);
        assert_eq!(align_if_outside(100.0, 100.0, 190.0, 20.0, 300.0), 190.0);
        assert_eq!(align_if_outside(100.0, 100.0, 260.0, 20.0, 300.0), 260.0);
        assert_eq!(align_if_outside(100.0, 100.0, 160.0, 180.0, 300.0), 160.0);
        assert_eq!(align_if_outside(100.0, 100.0, 350.0, 20.0, 300.0), 300.0);
        assert_eq!(align_if_outside(350.0, 100.0, 360.0, 20.0, 300.0), 300.0);
    }

    /// 真实 BSN hierarchy 中的 descendant 事件通过最近 Viewport 修改原生 ScrollPosition。
    #[test]
    fn nearest_viewport_handles_descendant_event() {
        let mut app = scene_app();
        app.add_plugins(WidgetryScrollAreaPlugin);
        app.world_mut()
            .spawn_scene(bsn! {
                ScrollAreaConfig { axis: ScrollAxis::Vertical }
                Children [(
                    Node { overflow: { ScrollAxis::Vertical.overflow() }, scrollbar_width: 0.0 }
                    ScrollArea
                    WidgetryScrollAreaViewport
                    Children [(
                        WidgetryScrollAreaContent
                        Children [(
                            Node
                            Name::new("outer target")
                        )]
                    )]
                )]
            })
            .unwrap();
        let viewport = app
            .world_mut()
            .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
            .single(app.world())
            .unwrap();
        let target = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(entity, name)| (name.as_str() == "outer target").then_some(entity))
            .unwrap();
        assert!(app.world().get::<ScrollArea>(viewport).is_some());
        let content_entity = app.world().get::<ChildOf>(target).unwrap().parent();
        assert!(
            app.world()
                .get::<WidgetryScrollAreaContent>(content_entity)
                .is_some()
        );
        assert_eq!(
            app.world().get::<ChildOf>(content_entity).unwrap().parent(),
            viewport
        );
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::splat(100.0),
            content_size: Vec2::splat(400.0),
            ..default()
        });
        app.world_mut().entity_mut(target).insert((
            ComputedNode {
                size: Vec2::splat(20.0),
                ..default()
            },
            UiGlobalTransform::from_xy(50.0, 170.0),
        ));
        app.world_mut()
            .entity_mut(viewport)
            .insert(UiGlobalTransform::from_xy(50.0, 50.0));
        assert_eq!(
            app.world().get::<Node>(viewport).unwrap().overflow.y,
            OverflowAxis::Scroll
        );
        assert!(app.world().get::<Node>(target).is_some());
        assert!(app.world().get::<UiGlobalTransform>(target).is_some());
        app.world_mut()
            .trigger(WidgetryScrollIntoView { entity: target });
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().0,
            Vec2::new(0.0, 160.0)
        );
    }

    /// nested ScrollArea 的目标只滚动内层 Viewport，外层 ScrollPosition 保持原值。
    #[test]
    fn nested_scroll_area_only_changes_nearest_viewport() {
        let mut app = scene_app();
        app.add_plugins(WidgetryScrollAreaPlugin);
        app.world_mut()
            .spawn_scene(bsn! {
                ScrollAreaConfig { axis: ScrollAxis::Vertical }
                Children [(
                    Node { overflow: { ScrollAxis::Vertical.overflow() } }
                    ScrollArea
                    WidgetryScrollAreaViewport
                    Name::new("outer viewport")
                    Children [(
                        WidgetryScrollAreaContent
                        Children [(
                            ScrollAreaConfig { axis: ScrollAxis::Vertical }
                            Children [(
                                Node { overflow: { ScrollAxis::Vertical.overflow() } }
                                ScrollArea
                                WidgetryScrollAreaViewport
                                Name::new("inner viewport")
                                Children [(
                                    WidgetryScrollAreaContent
                                    Children [(Node Name::new("nested target"))]
                                )]
                            )]
                        )]
                    )]
                )]
            })
            .unwrap();
        let named = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .map(|(entity, name)| (name.as_str().to_owned(), entity))
            .collect::<Vec<_>>();
        let outer = named
            .iter()
            .find(|(name, _)| name == "outer viewport")
            .unwrap()
            .1;
        let inner = named
            .iter()
            .find(|(name, _)| name == "inner viewport")
            .unwrap()
            .1;
        let target = named
            .iter()
            .find(|(name, _)| name == "nested target")
            .unwrap()
            .1;
        for viewport in [outer, inner] {
            app.world_mut().entity_mut(viewport).insert((
                ComputedNode {
                    size: Vec2::splat(100.0),
                    content_size: Vec2::splat(400.0),
                    ..default()
                },
                UiGlobalTransform::from_xy(50.0, 50.0),
            ));
        }
        app.world_mut().entity_mut(target).insert((
            ComputedNode {
                size: Vec2::splat(20.0),
                ..default()
            },
            UiGlobalTransform::from_xy(50.0, 170.0),
        ));
        app.world_mut()
            .trigger(WidgetryScrollIntoView { entity: target });
        assert_eq!(app.world().get::<ScrollPosition>(inner).unwrap().0.y, 160.0);
        assert_eq!(app.world().get::<ScrollPosition>(outer).unwrap().0.y, 0.0);
    }

    /// pointer press 使 root 获得 focus 后，官方 keyboard dispatch 应把方向键送到 root 并更新 Viewport。
    #[test]
    fn focused_keyboard_input_updates_viewport_scroll_position() {
        let mut app = scene_app();
        app.init_resource::<bevy::ui::UiScale>();
        app.add_message::<KeyboardInput>()
            .add_systems(
                PreUpdate,
                dispatch_focused_input::<KeyboardInput>.in_set(InputFocusSystems::Dispatch),
            )
            .add_plugins(WidgetryScrollAreaPlugin);
        let window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                ScrollAreaConfig { axis: ScrollAxis::Vertical }
                TabIndex(-1)
                Children [(
                    Node { overflow: { ScrollAxis::Vertical.overflow() } }
                    ScrollArea
                    WidgetryScrollAreaViewport
                    Children [(WidgetryScrollAreaContent)]
                )]
            })
            .unwrap()
            .id();
        let viewport = app
            .world_mut()
            .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
            .single(app.world())
            .unwrap();
        app.world_mut().entity_mut(viewport).insert(ComputedNode {
            size: Vec2::splat(100.0),
            content_size: Vec2::splat(400.0),
            ..default()
        });
        app.update();
        app.world_mut().trigger(primary_press(root));
        app.update();
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::ArrowDown,
            logical_key: Key::ArrowDown,
            state: ButtonState::Pressed,
            text: None,
            repeat: false,
            window,
        });
        app.update();
        assert_eq!(
            app.world().get::<ScrollPosition>(viewport).unwrap().0,
            Vec2::new(0.0, 100.0)
        );
    }
}
