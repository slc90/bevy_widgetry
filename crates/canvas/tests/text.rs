#![allow(clippy::unwrap_used, clippy::disallowed_macros)]

use bevy::prelude::*;
use bevy_widgetry_canvas::*;

#[test]
fn text_has_real_font_metrics_and_round_trips_as_a_distinct_document_item() {
    let mut world = World::new();
    let document = world.spawn(CanvasDocument).id();
    let mut style = CanvasTextStyle {
        content: "iiii".to_owned(),
        font_size: 25.0,
        color: Color::WHITE,
        align: CanvasTextAlign::Start,
    };
    let text =
        CanvasDocument::create_text(&mut world, document, style.clone(), Transform::IDENTITY)
            .unwrap();
    let narrow = world.get::<CanvasText>(text).unwrap().size();
    assert!(narrow.min_element() > 0.0);
    assert_eq!(
        CanvasDocument::draw_order(&world, document).unwrap(),
        [text]
    );
    assert!(
        CanvasDocument::shape_order(&mut world, document)
            .unwrap()
            .is_empty()
    );
    let wide = CanvasDocument::create_text(
        &mut world,
        document,
        CanvasTextStyle {
            content: "WWWW".into(),
            ..style.clone()
        },
        Transform::IDENTITY,
    )
    .unwrap();
    assert!(world.get::<CanvasText>(wide).unwrap().size().x > narrow.x);
    style.content = "WWWW\n中<&".to_owned();
    style.align = CanvasTextAlign::End;
    style.font_size = 32.0;
    style.color = Color::srgb(0.8, 0.2, 0.1);
    CanvasDocument::update_text(&mut world, document, text, style.clone()).unwrap();
    let updated = world.get::<CanvasText>(text).unwrap();
    assert_eq!(updated.style(), &style);
    let mut registry = bevy::reflect::TypeRegistry::new();
    registry.register::<CanvasText>();
    let reflected = serde_json::to_value(bevy::reflect::serde::TypedReflectSerializer::new(
        updated, &registry,
    ))
    .unwrap();
    assert_eq!(reflected["style"]["content"], style.content);
    assert!(updated.size().x > narrow.x && updated.size().y > narrow.y * 2.0);
    let directory = tempfile::tempdir().unwrap();
    let svg = directory.path().join("text.svg");
    CanvasDocument::export_svg(&mut world, document, &svg, None).unwrap();
    let svg = std::fs::read_to_string(svg).unwrap();
    assert!(svg.contains("<text") && svg.contains("中&lt;&amp;"));
    assert!(CanvasDocument::undo(&mut world, document).unwrap());
    assert_eq!(world.get::<CanvasText>(text).unwrap().size(), narrow);
    assert!(CanvasDocument::redo(&mut world, document).unwrap());
    assert_eq!(world.get::<CanvasText>(text).unwrap().style(), &style);
    style.font_size = f32::NAN;
    assert!(CanvasDocument::update_text(&mut world, document, text, style).is_err());
    assert_eq!(
        world.get::<CanvasText>(text).unwrap().style().font_size,
        32.0
    );
}

#[test]
fn text_copy_keeps_layer_group_and_local_pose_with_independent_history() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let layer = CanvasDocument::create_layer(&mut world, doc, 1, "Labels").unwrap();
    let text = CanvasDocument::create_text_in_layer(
        &mut world,
        doc,
        layer,
        CanvasTextStyle::default(),
        Transform::from_xyz(40., 0., 0.),
    )
    .unwrap();
    let first = CanvasDocument::duplicate_text(&mut world, doc, text).unwrap();
    let group = CanvasDocument::group_items(&mut world, doc, &[text, first], "Caption").unwrap();
    let copy = CanvasDocument::duplicate_text(&mut world, doc, text).unwrap();
    assert_eq!(world.get::<ChildOf>(copy).unwrap().parent(), group);
    let expected = *world.get::<Transform>(text).unwrap();
    assert_eq!(world.get::<Transform>(copy), Some(&expected));
    let moved = Transform::from_xyz(12., 20., 0.);
    CanvasDocument::update_transform(&mut world, doc, copy, moved).unwrap();
    assert_eq!(world.get::<Transform>(text), Some(&expected));
    CanvasDocument::remove_text(&mut world, doc, copy).unwrap();
    assert!(world.get::<ChildOf>(copy).is_none());
    assert!(CanvasDocument::undo(&mut world, doc).unwrap());
    assert_eq!(world.get::<ChildOf>(copy).unwrap().parent(), group);
    assert_eq!(world.get::<Transform>(copy), Some(&moved));
    CanvasDocument::ungroup(&mut world, doc, group).unwrap();
    assert_eq!(
        CanvasDocument::container_of(&world, doc, copy).unwrap(),
        CanvasContainer::Layer(layer)
    );
}

#[allow(dead_code)]
mod common;
#[path = "common/coordinates.rs"]
mod coordinates;

#[test]
fn click_places_a_label_and_both_views_project_committed_edits() {
    use bevy::picking::pointer::*;
    let id = PointerId::Mouse;
    let (mut app, a, b, _, _, base) = common::fixture(id, 1., 1.);
    let doc = CanvasView::document(app.world(), a).unwrap().unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), a, CanvasMode::Text).unwrap();
    let point = Vec2::new(42., 24.);
    let at = coordinates::at_document(&app, a, &base, point);
    for action in [
        PointerAction::Move { delta: Vec2::ONE },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        common::input(&mut app, id, &at, action);
    }
    let item = CanvasView::single_selection(app.world(), a)
        .unwrap()
        .unwrap();
    assert_eq!(
        CanvasView::mode(app.world(), a).unwrap(),
        CanvasMode::Select
    );
    assert!(
        app.world()
            .get::<Transform>(item)
            .unwrap()
            .translation
            .truncate()
            .abs_diff_eq(point, 0.01)
    );
    let mut draft = app.world().get::<CanvasText>(item).unwrap().style().clone();
    draft.content = "Updated".to_owned();
    assert_eq!(glyph_meshes(app.world_mut()).len(), 8);
    CanvasDocument::update_text(app.world_mut(), doc, item, draft).unwrap();
    common::update(&mut app);
    assert_eq!(glyph_meshes(app.world_mut()).len(), 14);
    let size = app.world().get::<CanvasText>(item).unwrap().size();
    for (offset, expected) in [
        (size * 0.47, Some(item)),
        (Vec2::new(0., size.y * 1.0), None),
    ] {
        let at = coordinates::at_document(&app, b, &base, point + offset);
        for action in [
            PointerAction::Move { delta: Vec2::ONE },
            PointerAction::Press(PointerButton::Primary),
            PointerAction::Release(PointerButton::Primary),
        ] {
            common::input(&mut app, id, &at, action);
        }
        assert_eq!(
            CanvasView::single_selection(app.world(), b).unwrap(),
            expected
        );
    }
}

fn glyph_meshes(world: &mut World) -> Vec<(AssetId<Mesh>, Vec2)> {
    world
        .query::<(&Name, &Mesh2d, &GlobalTransform)>()
        .iter(world)
        .filter(|(name, _, _)| name.as_str() == "CanvasGlyph")
        .map(|(_, mesh, pose)| (mesh.0.id(), pose.translation().truncate()))
        .collect()
}

#[test]
fn views_share_text_meshes_until_zoom_or_group_scale_requires_finer_curves() {
    let (mut app, a, b, shape, _, _) =
        common::fixture(bevy::picking::pointer::PointerId::Mouse, 1., 1.);
    let doc = CanvasView::document(app.world(), a).unwrap().unwrap();
    let style = CanvasTextStyle {
        content: "O O\n回".into(),
        ..default()
    };
    let text =
        CanvasDocument::create_text(app.world_mut(), doc, style.clone(), Transform::IDENTITY)
            .unwrap();
    let group =
        CanvasDocument::group_items(app.world_mut(), doc, &[shape, text], "Glyphs").unwrap();
    common::update(&mut app);
    let before = glyph_meshes(app.world_mut());
    assert_eq!(before.len(), 6);
    let ids = |glyphs: &[(AssetId<Mesh>, Vec2)]| {
        glyphs
            .iter()
            .map(|(id, _)| *id)
            .collect::<std::collections::HashSet<_>>()
    };
    assert_eq!(ids(&before).len(), 2);
    for id in ids(&before) {
        let assets = app.world().resource::<Assets<Mesh>>();
        assert_outline_has_counter(assets.get(id).unwrap());
    }
    let group_origin = app
        .world()
        .get::<Transform>(group)
        .unwrap()
        .translation
        .truncate();
    CanvasView::set_zoom_in_world(app.world_mut(), b, 4.).unwrap();
    common::update(&mut app);
    let zoomed = glyph_meshes(app.world_mut());
    assert_eq!(ids(&zoomed).len(), 4);
    assert!(before.iter().all(|glyph| zoomed.contains(glyph)));
    assert!(
        zoomed
            .iter()
            .all(|(_, position)| before.iter().any(|(_, previous)| position == previous))
    );
    CanvasDocument::update_group_transform(
        app.world_mut(),
        doc,
        group,
        Transform::from_scale(Vec3::new(4., 4., 1.)),
    )
    .unwrap();
    common::update(&mut app);
    let scaled = glyph_meshes(app.world_mut());
    assert_eq!(ids(&scaled).len(), 4);
    assert!(scaled.iter().all(|(_, position)| {
        before.iter().any(|(_, previous)| {
            position.abs_diff_eq((*previous - group_origin * Vec2::new(1., -1.)) * 4., 0.001)
        })
    }));
    CanvasDocument::update_transform(
        app.world_mut(),
        doc,
        text,
        Transform::from_scale(Vec3::new(6., 0.5, 1.)),
    )
    .unwrap();
    common::update(&mut app);
    assert!(ids(&glyph_meshes(app.world_mut())).is_disjoint(&ids(&scaled)));
    for content in ["   ", ""] {
        CanvasDocument::update_text(
            app.world_mut(),
            doc,
            text,
            CanvasTextStyle {
                content: content.into(),
                ..style.clone()
            },
        )
        .unwrap();
        common::update(&mut app);
        assert!(glyph_meshes(app.world_mut()).is_empty());
        assert_eq!(
            app.world().get::<CanvasText>(text).unwrap().size().x > 0.,
            !content.is_empty()
        );
    }
}

fn assert_outline_has_counter(mesh: &Mesh) {
    let vertices = mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .and_then(bevy::mesh::VertexAttributeValues::as_float3)
        .unwrap();
    let vertices = vertices
        .iter()
        .map(|v| Vec3::from_array(*v).truncate())
        .collect::<Vec<_>>();
    let bounds = Rect {
        min: vertices
            .iter()
            .copied()
            .fold(Vec2::splat(f32::INFINITY), Vec2::min),
        max: vertices
            .iter()
            .copied()
            .fold(Vec2::splat(f32::NEG_INFINITY), Vec2::max),
    };
    let indices = mesh.indices().unwrap().iter().collect::<Vec<_>>();
    let filled = |point: Vec2| {
        indices.as_chunks::<3>().0.iter().any(|triangle| {
            let [a, b, c] = [
                vertices[triangle[0]],
                vertices[triangle[1]],
                vertices[triangle[2]],
            ];
            let edges = [
                (b - a).perp_dot(point - a),
                (c - b).perp_dot(point - b),
                (a - c).perp_dot(point - c),
            ];
            edges.iter().all(|v| *v >= 0.) || edges.iter().all(|v| *v <= 0.)
        })
    };
    assert!(!filled(bounds.center()));
    assert!((1..20).any(|step| filled(Vec2::new(
        bounds.min.x + bounds.width() * step as f32 / 40.,
        bounds.center().y
    ))));
}

#[test]
fn blank_lines_and_unicode_breaks_keep_their_svg_baselines() {
    let mut world = World::new();
    let doc = world.spawn(CanvasDocument).id();
    let item = CanvasDocument::create_text(
        &mut world,
        doc,
        CanvasTextStyle {
            content: String::new(),
            ..default()
        },
        Transform::IDENTITY,
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("breaks.svg");
    let region = Some(Rect::new(-80., -80., 80., 80.));
    CanvasDocument::export_svg(&mut world, doc, &path, region).unwrap();
    CanvasDocument::update_text(
        &mut world,
        doc,
        item,
        CanvasTextStyle {
            content: "A\r\n\nB\u{2028}C".into(),
            ..default()
        },
    )
    .unwrap();
    CanvasDocument::export_svg(&mut world, doc, &path, region).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert_eq!(svg.matches("<tspan ").count(), 4);
    for line in ["A", "B", "C"] {
        assert!(svg.contains(&format!(">{line}</tspan>")));
    }
}

#[test]
fn text_tool_places_in_the_edited_group_in_document_coordinates() {
    use bevy::picking::pointer::*;
    let id = PointerId::Mouse;
    let (mut app, view, _, shape, _, base) = common::fixture(id, 1., 1.);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let first = CanvasDocument::create_text(
        app.world_mut(),
        doc,
        CanvasTextStyle::default(),
        Transform::IDENTITY,
    )
    .unwrap();
    let group =
        CanvasDocument::group_items(app.world_mut(), doc, &[shape, first], "Labels").unwrap();
    CanvasDocument::update_group_transform(
        app.world_mut(),
        doc,
        group,
        Transform::from_xyz(7., 4., 0.)
            .with_rotation(Quat::from_rotation_z(0.4))
            .with_scale(Vec3::new(2., 2., 1.)),
    )
    .unwrap();
    CanvasView::enter_group_in_world(app.world_mut(), view, group).unwrap();
    CanvasView::set_mode_in_world(app.world_mut(), view, CanvasMode::Text).unwrap();
    let point = Vec2::new(-45., 20.);
    let at = coordinates::at_document(&app, view, &base, point);
    for action in [
        PointerAction::Move { delta: Vec2::ONE },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        common::input(&mut app, id, &at, action);
    }
    let item = CanvasView::single_selection(app.world(), view)
        .unwrap()
        .unwrap();
    assert_eq!(
        CanvasDocument::container_of(app.world(), doc, item).unwrap(),
        CanvasContainer::Group(group)
    );
    let pose = CanvasDocument::document_transform(app.world(), doc, item).unwrap();
    assert!(
        pose.transform_point3(Vec3::X)
            .truncate()
            .abs_diff_eq(point + Vec2::X, 0.001)
    );
}

#[test]
fn overhanging_glyphs_are_selectable_and_batch_move_with_shapes() {
    use bevy::picking::pointer::*;
    let id = PointerId::Mouse;
    let (mut app, view, _, shape, _, base) = common::fixture(id, 1., 1.);
    let doc = CanvasView::document(app.world(), view).unwrap().unwrap();
    let style = CanvasTextStyle {
        content: "jW".into(),
        font_size: 80.,
        ..default()
    };
    let item =
        CanvasDocument::create_text(app.world_mut(), doc, style, Transform::IDENTITY).unwrap();
    let advance = app.world().get::<CanvasText>(item).unwrap().size();
    let bounds = app.world().get::<CanvasText>(item).unwrap().local_bounds();
    assert!(bounds.min.x < -advance.x * 0.5 && bounds.max.x > advance.x * 0.5);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ink.svg");
    CanvasDocument::export_svg(app.world_mut(), doc, &path, None).unwrap();
    let svg = std::fs::read_to_string(path).unwrap();
    assert!(svg.contains(&format!(
        "viewBox=\"{} {} {} {}\"",
        bounds.min.x,
        bounds.min.y,
        bounds.width(),
        bounds.height()
    )));
    let at = coordinates::at_document(&app, view, &base, Vec2::new(-advance.x * 0.5 - 3., 12.));
    for action in [
        PointerAction::Move { delta: Vec2::ONE },
        PointerAction::Press(PointerButton::Primary),
        PointerAction::Release(PointerButton::Primary),
    ] {
        common::input(&mut app, id, &at, action);
    }
    assert_eq!(
        CanvasView::single_selection(app.world(), view).unwrap(),
        Some(item)
    );
    let moved = Transform::from_xyz(20., 30., 0.);
    CanvasDocument::update_transforms(app.world_mut(), doc, &[(shape, moved), (item, moved)])
        .unwrap();
    assert_eq!(app.world().get::<Transform>(item), Some(&moved));
    assert!(CanvasDocument::undo(app.world_mut(), doc).unwrap());
    assert_eq!(
        app.world().get::<Transform>(shape),
        Some(&Transform::IDENTITY)
    );
    assert_eq!(
        app.world().get::<Transform>(item),
        Some(&Transform::IDENTITY)
    );
}
