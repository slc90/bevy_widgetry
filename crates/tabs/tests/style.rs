#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::input_focus::{FocusCause, InputFocus};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_core::{foreground::ResolvedForeground, text::WidgetryText};
use bevy_widgetry_tabs::{
    TabsOrientation, WidgetryTab, WidgetryTabColorOverrides, WidgetryTabs,
    WidgetryTabsColorOverrides, WidgetryTabsPlugin,
};
use bevy_widgetry_test_utils::{
    ErrorCapture, add_ui_plugins, advance_until, primary_press, primary_release, scene_app,
    spawn_ui_camera,
};
use bevy_widgetry_theme::WidgetryThemeMode;

const TITLES: [&str; 3] = [
    "A",
    "Some settings",
    "Advanced rendering and graphics device compatibility settings",
];

#[derive(Resource, Default)]
struct Changes(usize);

fn fixture(orientation: TabsOrientation, scale: f32, layout: bool) -> (App, Entity) {
    fixture_title(orientation, scale, layout, false)
}

fn fixture_title(
    orientation: TabsOrientation,
    scale: f32,
    layout: bool,
    direct: bool,
) -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryTabsPlugin);
    app.init_resource::<Changes>().add_observer(
        |_: On<bevy::ui_widgets::ValueChange<usize>>, mut changes: ResMut<Changes>| {
            changes.0 += 1;
        },
    );
    if layout {
        add_ui_plugins(&mut app);
        spawn_ui_camera(&mut app, UVec2::new(1400, 900), scale);
        app.add_plugins(bevy_widgetry_asset::WidgetryAssetPlugin);
        let font = app
            .world()
            .resource::<AssetServer>()
            .load::<Font>(bevy_widgetry_asset::BuiltinFont::Default.path());
        advance_until(
            &mut app,
            std::time::Duration::from_secs(10),
            "Tabs built-in font",
            |world| world.resource::<Assets<Font>>().contains(&font),
        )
        .unwrap();
        bevy_widgetry_core::WidgetryAppExt::set_default_font(
            &mut app,
            bevy::text::FontSource::Handle(font),
        );
    }
    let items: Vec<Box<dyn Scene>> = TITLES
        .into_iter()
        .map(|title| {
            let header: Box<dyn Scene> = if direct {
                Box::new(bsn! { Text(title) WidgetryText })
            } else {
                Box::new(bsn! { Children [Text(title) WidgetryText] })
            };
            let content: Box<dyn Scene> =
                Box::new(bsn! { Node BackgroundColor(Color::srgb(0.8, 0.1, 0.2)) });
            Box::new(bsn! { @WidgetryTab { @header: {Some(header)}, @content: {Some(content)} } })
                as Box<dyn Scene>
        })
        .collect();
    let items: Box<dyn SceneList> = Box::new(items);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTabs { @orientation: orientation, @children: {Some(items)} }
            Node { position_type: PositionType::Absolute, width: {if orientation == TabsOrientation::Horizontal {px(1200)} else {px(600)}}, height: px(240) }
        })
        .unwrap()
        .id();
    for _ in 0..4 {
        app.update();
    }
    if layout {
        let texts: Vec<_> = (0..3)
            .map(|index| {
                text_and_ellipsis(
                    app.world(),
                    WidgetryTabs::header(app.world(), root, index).unwrap(),
                )
                .0
            })
            .collect();
        advance_until(
            &mut app,
            std::time::Duration::from_secs(10),
            "Tabs title layout",
            |world| {
                texts.iter().all(|entity| {
                    world
                        .get::<bevy::text::TextLayoutInfo>(*entity)
                        .is_some_and(|info| info.size.x > 0.0)
                })
            },
        )
        .unwrap();
        app.update();
        app.update();
    }
    (app, root)
}

fn text_and_ellipsis(world: &World, header: Entity) -> (Entity, Entity) {
    let text = if world.get::<Text>(header).is_some() {
        header
    } else {
        world
            .get::<Children>(header)
            .unwrap()
            .iter()
            .find(|entity| world.get::<Text>(*entity).is_some_and(|text| text.0 != "…"))
            .unwrap()
    };
    let parent = world.get::<ChildOf>(header).unwrap().parent();
    let ellipsis = world
        .get::<Children>(parent)
        .unwrap()
        .iter()
        .find(|entity| {
            world
                .get::<bevy_widgetry_core::color::WidgetryStyleOwner<WidgetryTab>>(*entity)
                .is_some_and(|owner| owner.entity == header)
        })
        .unwrap();
    (text, ellipsis)
}

fn bounds(world: &World, entity: Entity) -> Rect {
    let size = world.get::<ComputedNode>(entity).unwrap().size();
    let position = world.get::<UiGlobalTransform>(entity).unwrap().translation;
    Rect::from_center_size(position, size)
}

#[test]
fn palettes_overrides_interactions_and_focus_preserve_selection_and_page_style() {
    let (mut app, root) = fixture(TabsOrientation::Horizontal, 1.0, false);
    let selected = WidgetryTabs::header(app.world(), root, 0).unwrap();
    let other = WidgetryTabs::header(app.world(), root, 1).unwrap();
    for theme in [
        WidgetryThemeMode::Dark,
        WidgetryThemeMode::Light,
        WidgetryThemeMode::PinkDream,
        WidgetryThemeMode::KamuriViolet,
    ] {
        WidgetryThemeMode::set_in_world(app.world_mut(), theme).unwrap();
        app.update();
        let palette = theme.colors().tabs;
        assert_eq!(
            app.world().get::<BackgroundColor>(selected).unwrap().0,
            palette.header.selected.normal.background
        );
        assert_eq!(
            app.world().get::<ResolvedForeground>(selected).unwrap().0,
            palette.header.selected.normal.foreground
        );
        app.world_mut().entity_mut(other).insert(Hovered(true));
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(other).unwrap().0,
            palette.header.unselected.hovered.background
        );
        app.world_mut().entity_mut(other).insert(Pressed);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(other).unwrap().0,
            palette.header.unselected.pressed.background
        );
        app.world_mut()
            .entity_mut(other)
            .insert(InteractionDisabled);
        app.world_mut()
            .entity_mut(selected)
            .insert(InteractionDisabled);
        app.update();
        assert_eq!(
            app.world().get::<BackgroundColor>(selected).unwrap().0,
            palette.header.selected.disabled.background
        );
        assert_eq!(
            app.world().get::<BackgroundColor>(other).unwrap().0,
            palette.header.unselected.disabled.background
        );
        assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
        app.world_mut()
            .entity_mut(selected)
            .remove::<InteractionDisabled>();
        app.world_mut()
            .entity_mut(other)
            .remove::<(InteractionDisabled, Pressed)>();
        app.world_mut().entity_mut(other).insert(Hovered(false));
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(selected, FocusCause::Navigated);
        app.update();
        assert_eq!(
            app.world().get::<BorderColor>(selected).unwrap().top,
            palette.header.focus_border
        );
        app.world_mut().resource_mut::<InputFocus>().clear();
        app.update();
    }
    let mut group = WidgetryTabsColorOverrides::default();
    group.header.selected.normal.background = Some(Color::srgb(0.1, 0.6, 0.2));
    WidgetryTabsColorOverrides::set_in_world(app.world_mut(), root, group.clone()).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(selected).unwrap().0,
        group.header.selected.normal.background.unwrap()
    );
    let mut local = WidgetryTabColorOverrides::default();
    local.selected.normal.background = Some(Color::srgb(0.3, 0.2, 0.8));
    WidgetryTabColorOverrides::set_in_world(app.world_mut(), selected, local.clone()).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(selected).unwrap().0,
        local.selected.normal.background.unwrap()
    );
    assert!(
        !WidgetryTabColorOverrides::set_in_world(app.world_mut(), selected, local.clone()).unwrap()
    );
    local.selected.normal.background = Some(Color::srgba(0.0, 0.0, 0.0, f32::NAN));
    assert_eq!(
        WidgetryTabColorOverrides::set_in_world(app.world_mut(), selected, local)
            .unwrap_err()
            .severity(),
        bevy::ecs::error::Severity::Error
    );
    WidgetryTabColorOverrides::clear_in_world(app.world_mut(), selected).unwrap();
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(selected).unwrap().0,
        group.header.selected.normal.background.unwrap()
    );
    let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
    let content = app.world().get::<Children>(page).unwrap()[0];
    assert_eq!(
        app.world().get::<BackgroundColor>(content).unwrap().0,
        Color::srgb(0.8, 0.1, 0.2)
    );
    assert_eq!(app.world().resource::<Changes>().0, 0);
}

#[test]
fn header_pointer_press_projects_pressed_colors_without_changing_selection() {
    let (mut app, root) = fixture(TabsOrientation::Horizontal, 1.0, false);
    let header = WidgetryTabs::header(app.world(), root, 1).unwrap();
    app.world_mut().trigger(primary_press(header));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(header).is_some());
    app.update();
    assert_eq!(
        app.world().get::<BackgroundColor>(header).unwrap().0,
        WidgetryThemeMode::Dark
            .colors()
            .tabs
            .header
            .unselected
            .pressed
            .background
    );
    assert_eq!(WidgetryTabs::selected_index(app.world(), root).unwrap(), 0);
    app.world_mut().trigger(primary_release(header));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(header).is_none());
    app.world_mut()
        .entity_mut(header)
        .insert(InteractionDisabled);
    app.world_mut().trigger(primary_press(header));
    app.world_mut().flush();
    assert!(app.world().get::<Pressed>(header).is_none());
    assert_eq!(app.world().resource::<Changes>().0, 0);
}

#[test]
fn classical_layout_is_flush_and_vertical_titles_elide_without_changing_the_source() {
    for orientation in [TabsOrientation::Horizontal, TabsOrientation::Vertical] {
        for scale in [1.0, 1.25, 2.0] {
            let (mut app, root) = fixture(orientation, scale, true);
            let headers: Vec<_> = (0..3)
                .map(|index| WidgetryTabs::header(app.world(), root, index).unwrap())
                .collect();
            let rectangles: Vec<_> = headers
                .iter()
                .map(|entity| bounds(app.world(), *entity))
                .collect();
            let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
            let panel = app.world().get::<ChildOf>(page).unwrap().parent();
            let panel_rect = bounds(app.world(), panel);
            if orientation == TabsOrientation::Horizontal {
                assert!(rectangles[0].width() < rectangles[1].width());
                assert!((panel_rect.min.y - rectangles[0].max.y).abs() <= 2.0 * scale);
            } else {
                assert!(
                    rectangles
                        .iter()
                        .all(|rect| (rect.width() - rectangles[0].width()).abs() < 1.0
                            && (rect.height() - rectangles[0].height()).abs() < 1.0)
                );
                assert!(rectangles[0].width() <= 182.0 * scale);
                assert!((panel_rect.min.x - rectangles[0].max.x).abs() <= 2.0 * scale);
                let (text, ellipsis) = text_and_ellipsis(app.world(), headers[2]);
                assert_eq!(app.world().get::<Text>(text).unwrap().0, TITLES[2]);
                assert_eq!(
                    app.world().get::<TextLayout>(text).unwrap().linebreak,
                    bevy::text::LineBreak::NoWrap
                );
                assert_eq!(
                    app.world().get::<Node>(ellipsis).unwrap().display,
                    Display::Flex
                );
                let ellipsis_bounds = bounds(app.world(), ellipsis);
                // The mask must cover the entire trailing edge, not leave source glyphs after it.
                assert!((ellipsis_bounds.max.x - rectangles[2].max.x).abs() <= scale);
                app.world_mut().get_mut::<Text>(text).unwrap().0 = "Short".into();
                for _ in 0..3 {
                    app.update();
                }
                assert_eq!(
                    app.world().get::<Node>(ellipsis).unwrap().display,
                    Display::None
                );
            }
        }
    }
}

#[test]
fn missing_managed_outputs_report_once_and_recover_after_reinsertion() {
    let (mut app, root) = fixture(TabsOrientation::Horizontal, 1.0, false);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::new());
    });
    let header = WidgetryTabs::header(app.world(), root, 0).unwrap();
    let page = WidgetryTabs::page(app.world(), root, 0).unwrap();
    let panel = app.world().get::<ChildOf>(page).unwrap().parent();
    let errors = ErrorCapture::default();
    for entity in [header, panel] {
        app.world_mut()
            .entity_mut(entity)
            .remove::<BackgroundColor>();
        errors.run(|| app.update());
        let captured = errors.take();
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].severity(), bevy::ecs::error::Severity::Error);
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        app.world_mut()
            .entity_mut(entity)
            .insert(BackgroundColor::default());
        errors.run(|| app.update());
        assert!(errors.take().is_empty());
        assert_eq!(
            app.world().get::<BackgroundColor>(entity).unwrap().0,
            WidgetryThemeMode::Dark.colors().tabs.panel_background
        );
    }
    let ellipsis = text_and_ellipsis(app.world(), header).1;
    app.world_mut()
        .entity_mut(ellipsis)
        .remove::<ResolvedForeground>();
    errors.run(|| app.update());
    assert_eq!(errors.take().len(), 1);
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
    app.world_mut()
        .entity_mut(ellipsis)
        .insert(ResolvedForeground::default());
    errors.run(|| app.update());
    assert!(errors.take().is_empty());
    assert_eq!(
        app.world().get::<ResolvedForeground>(ellipsis).unwrap().0,
        WidgetryThemeMode::Dark
            .colors()
            .tabs
            .header
            .selected
            .normal
            .foreground
    );
}

#[test]
fn direct_and_nested_titles_clip_rendered_glyphs_with_transparent_colors_and_reflow() {
    for direct in [false, true] {
        let (mut app, root) = fixture_title(TabsOrientation::Vertical, 1.25, true, direct);
        let header = WidgetryTabs::header(app.world(), root, 2).unwrap();
        let (text, ellipsis) = text_and_ellipsis(app.world(), header);
        for alpha in [0.0, 0.5] {
            let mut colors = WidgetryTabColorOverrides::default();
            colors.unselected.normal.background = Some(Color::srgba(0.4, 0.1, 0.8, alpha));
            WidgetryTabColorOverrides::set_in_world(app.world_mut(), header, colors).unwrap();
            for _ in 0..3 {
                app.update();
            }
            assert_eq!(app.world().get::<Text>(text).unwrap().0, TITLES[2]);
            assert_eq!(
                app.world().get::<BackgroundColor>(ellipsis).unwrap().0,
                Color::NONE
            );
            let node = app.world().get::<ComputedNode>(text).unwrap();
            let origin = app
                .world()
                .get::<UiGlobalTransform>(text)
                .unwrap()
                .translation
                + node.content_box().min;
            let ellipsis_bounds = bounds(app.world(), ellipsis);
            let info = app.world().get::<bevy::text::TextLayoutInfo>(text).unwrap();
            assert!(!info.glyphs.is_empty());
            assert!(info.glyphs.iter().all(|glyph| origin.x
                + glyph.position.x
                + glyph.atlas_info.rect.width() * 0.5
                <= ellipsis_bounds.min.x + 0.5));
        }
        let clipped = app
            .world()
            .get::<bevy::text::TextLayoutInfo>(text)
            .unwrap()
            .glyphs
            .len();
        let bar = app.world().get::<Children>(root).unwrap()[0];
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(1400);
        app.world_mut().get_mut::<Node>(bar).unwrap().width = px(1100);
        for _ in 0..5 {
            app.update();
        }
        let complete = app
            .world()
            .get::<bevy::text::TextLayoutInfo>(text)
            .unwrap()
            .glyphs
            .len();
        assert!(complete > clipped);
        assert_eq!(
            app.world().get::<Node>(ellipsis).unwrap().display,
            Display::None
        );
        app.world_mut().get_mut::<Node>(bar).unwrap().width = px(180);
        for _ in 0..5 {
            app.update();
        }
        assert_eq!(
            app.world()
                .get::<bevy::text::TextLayoutInfo>(text)
                .unwrap()
                .glyphs
                .len(),
            clipped
        );
        app.world_mut().get_mut::<Text>(text).unwrap().0 = "Short".into();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world().get::<Node>(ellipsis).unwrap().display,
            Display::None
        );
        app.world_mut().get_mut::<Text>(text).unwrap().0 = TITLES[2].into();
        for _ in 0..4 {
            app.update();
        }
        assert_eq!(
            app.world()
                .get::<bevy::text::TextLayoutInfo>(text)
                .unwrap()
                .glyphs
                .len(),
            clipped
        );
        let tick = app
            .world()
            .entity(text)
            .get_ref::<bevy::text::TextLayoutInfo>()
            .unwrap()
            .last_changed();
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world()
                .entity(text)
                .get_ref::<bevy::text::TextLayoutInfo>()
                .unwrap()
                .last_changed(),
            tick
        );
    }
}

#[test]
fn direct_horizontal_titles_keep_natural_width_with_absolute_header_children() {
    let (mut app, root) = fixture_title(TabsOrientation::Horizontal, 1.25, true, true);
    let first = WidgetryTabs::header(app.world(), root, 0).unwrap();
    let second = WidgetryTabs::header(app.world(), root, 1).unwrap();
    let before = bounds(app.world(), second).width();
    app.world_mut().spawn_scene(bsn! { Node { position_type: PositionType::Absolute, width: px(10), height: px(10) } ChildOf(second) }).unwrap();
    for _ in 0..5 {
        app.update();
    }
    assert!((bounds(app.world(), second).width() - before).abs() <= 1.0);
    assert!(bounds(app.world(), first).width() < before);
    app.world_mut().get_mut::<Text>(second).unwrap().0 = "Longer header label".into();
    for _ in 0..5 {
        app.update();
    }
    assert!(bounds(app.world(), second).width() > before);
}

#[test]
fn vertical_overflow_with_an_elided_title_converges_and_locates_the_selected_header() {
    let (mut app, root) = fixture(TabsOrientation::Vertical, 1.25, true);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(90);
    for _ in 0..8 {
        app.update();
    }
    let bar = app.world().get::<Children>(root).unwrap()[0];
    let children = app.world().get::<Children>(bar).unwrap().to_vec();
    let view = children[0];
    let scrollbar = children[1];
    for _ in 0..10 {
        app.update();
        assert_ne!(
            app.world().get::<Node>(scrollbar).unwrap().display,
            Display::None
        );
    }
    WidgetryTabs::set_selected_in_world(app.world_mut(), root, 2).unwrap();
    for _ in 0..8 {
        app.update();
    }
    let target = bounds(
        app.world(),
        WidgetryTabs::header(app.world(), root, 2).unwrap(),
    );
    let viewport = bounds(app.world(), view);
    assert!(
        target.min.y >= viewport.min.y - 1.0 && target.max.y <= viewport.max.y + 1.0,
        "target={target:?}, viewport={viewport:?}"
    );
}
