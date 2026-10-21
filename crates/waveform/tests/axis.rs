#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy_widgetry_test_utils::{benchmark::ui_app, spawn_ui_camera};
use bevy_widgetry_waveform::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use std::time::Duration;

#[derive(Default)]
struct Source {
    reads: AtomicUsize,
    pending: AtomicBool,
    replay: bool,
}
impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 100,
            unit: WaveformUnit::Volts,
            kind: if self.replay {
                WaveformSessionKind::Replay { total_frames: 6000 }
            } else {
                WaveformSessionKind::Live
            },
        })
    }
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if self.pending.load(Ordering::Relaxed) {
            return Ok(WaveformReadStatus::Pending);
        }
        out.channels_mut()[0].resize((request.range.end - request.range.start) as usize, 0.5);
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

fn fixture() -> (App, Entity, Arc<Source>) {
    let mut app = ui_app().unwrap();
    app.add_plugins((TransformPlugin, WaveformRenderPlugin));
    let camera = spawn_ui_camera(&mut app, UVec2::new(1000, 800), 1.25);
    let source = Arc::new(Source::default());
    let adapter: Arc<dyn WaveformSource> = source.clone();
    let root = app.world_mut().spawn_scene(bsn! {
        @Waveform { @source: {Some(adapter)}, @config: {WaveformConfig { visible_duration_ms: 10000, ..default() }} }
        template(move |_| Ok(UiTargetCamera(camera)))
        Node { width: px(500), height: px(200) }
    }).unwrap().id();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(35);
    for _ in 0..3 {
        app.update();
    }
    (app, root, source)
}

fn named_mesh(app: &mut App, name: &str) -> Handle<Mesh> {
    app.world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .find_map(|(candidate, mesh)| (candidate.as_str() == name).then(|| mesh.0.clone()))
        .unwrap()
}
fn positions(app: &App, mesh: &Handle<Mesh>) -> Vec<[f32; 3]> {
    let Some(VertexAttributeValues::Float32x3(values)) = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(mesh)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions required");
    };
    values.clone()
}
fn labels(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<(&Name, &Text, &Node)>()
        .iter(app.world())
        .filter(|(name, _, node)| {
            name.as_str() == "WaveformTimeLabel" && node.display != Display::None
        })
        .map(|(_, text, _)| text.0.clone())
        .collect()
}

#[test]
fn grid_and_axis_follow_the_committed_absolute_window_including_pending_changes() {
    let (mut app, root, source) = fixture();
    assert_eq!(labels(&mut app), ["26 s", "28 s", "30 s", "32 s", "34 s"]);
    let grid = named_mesh(&mut app, "WaveformTimeGrid");
    let material = app
        .world_mut()
        .query::<(&Name, &MeshMaterial2d<ColorMaterial>)>()
        .iter(app.world())
        .find_map(|(name, material)| {
            (name.as_str() == "WaveformTimeGrid").then(|| material.0.clone())
        })
        .unwrap();
    assert_eq!(
        app.world()
            .resource::<Assets<ColorMaterial>>()
            .get(&material)
            .unwrap()
            .alpha_mode,
        bevy::sprite_render::AlphaMode2d::Blend
    );
    let asset = app.world().resource::<Assets<Mesh>>().get(&grid).unwrap();
    assert!(
        asset
            .attributes()
            .all(|(_, values)| values.len() == asset.count_vertices()),
        "grid vertex attributes must have matching lengths"
    );
    let before = positions(&app, &grid);
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_millis(35100);
    app.update();
    let after = positions(&app, &grid);
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(&after) {
        assert!((a[0] - b[0] - 0.01).abs() < 0.00001);
    }
    source.pending.store(true, Ordering::Relaxed);
    Waveform::set_visible_duration_in_world(app.world_mut(), root, 30000).unwrap();
    app.update();
    assert_eq!(labels(&mut app), ["26 s", "28 s", "30 s", "32 s", "34 s"]);
    assert_eq!(positions(&app, &grid), after);
    source.pending.store(false, Ordering::Relaxed);
    app.update();
    assert_eq!(labels(&mut app), ["10 s", "20 s", "30 s"]);
    assert_eq!(
        app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .viewport_range(),
        510..3510
    );
}

#[derive(Resource, Default)]
struct Notices(Vec<bool>);
#[test]
fn grid_updates_notify_after_commit_without_changing_data_or_time() {
    let (mut app, root, source) = fixture();
    app.init_resource::<Notices>();
    app.add_observer(|event: On<WaveformGridChanged>, world: &World| {
        assert_eq!(
            Waveform::grid_visible(world, event.entity).unwrap(),
            event.visible
        );
    });
    app.add_observer(
        |event: On<WaveformGridChanged>, mut notices: ResMut<Notices>| {
            notices.0.push(event.visible)
        },
    );
    Waveform::set_running_in_world(app.world_mut(), root, false).unwrap();
    let wave = named_mesh(&mut app, "WaveformMesh");
    let grid = named_mesh(&mut app, "WaveformTimeGrid");
    let old = positions(&app, &wave);
    let revision = app.world().get::<WaveformRuntime>(root).unwrap().revision();
    let reads = source.reads.load(Ordering::Relaxed);
    assert!(Waveform::grid_visible(app.world(), root).unwrap());
    assert!(Waveform::set_grid_visible_in_world(app.world_mut(), root, false).unwrap());
    assert!(!Waveform::set_grid_visible_in_world(app.world_mut(), root, false).unwrap());
    app.update();
    assert_eq!(positions(&app, &grid), [[0.0; 3]; 3]);
    assert_eq!(positions(&app, &wave), old);
    assert_eq!(
        app.world().get::<WaveformRuntime>(root).unwrap().revision(),
        revision
    );
    assert_eq!(source.reads.load(Ordering::Relaxed), reads);
    Waveform::set_grid_visible(&mut app.world_mut().commands(), root, true);
    assert!(!Waveform::grid_visible(app.world(), root).unwrap());
    app.world_mut().flush();
    assert_eq!(app.world().resource::<Notices>().0, [false, true]);
    app.update();
    assert!(positions(&app, &grid).len() > 3);
    assert_eq!(source.reads.load(Ordering::Relaxed), reads);
    let other = app.world_mut().spawn_empty().id();
    assert!(Waveform::set_grid_visible_in_world(app.world_mut(), other, false).is_err());
}

#[test]
fn narrow_resize_and_ui_scale_never_overlap_visible_measured_labels() {
    let (mut app, root, _) = fixture();
    for (width, scale) in [(30.0, 1.0), (120.0, 1.0), (700.0, 1.0), (200.0, 1.5)] {
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(width);
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        for _ in 0..3 {
            app.update();
            let mut intervals = app
                .world_mut()
                .query::<(
                    &Name,
                    &ComputedNode,
                    &UiGlobalTransform,
                    &InheritedVisibility,
                )>()
                .iter(app.world())
                .filter(|(name, _, _, visible)| {
                    name.as_str() == "WaveformTimeLabel" && visible.get()
                })
                .map(|(_, node, transform, _)| {
                    (
                        transform.translation.x - node.size().x * 0.5,
                        transform.translation.x + node.size().x * 0.5,
                    )
                })
                .collect::<Vec<_>>();
            intervals.sort_by(|a, b| a.0.total_cmp(&b.0));
            assert!(
                intervals.windows(2).all(|pair| pair[0].1 <= pair[1].0),
                "{intervals:?}"
            );
        }
        if width >= 120.0 {
            assert!(
                app.world_mut()
                    .query::<(&Name, &InheritedVisibility)>()
                    .iter(app.world())
                    .any(|(name, visible)| name.as_str() == "WaveformTimeLabel" && visible.get()),
                "width={width}, scale={scale}; details={:?}",
                app.world_mut()
                    .query::<(
                        &Name,
                        &Text,
                        &ComputedNode,
                        &UiGlobalTransform,
                        &Visibility,
                        &bevy::text::TextLayoutInfo
                    )>()
                    .iter(app.world())
                    .filter(|(name, ..)| name.as_str() == "WaveformTimeLabel")
                    .map(|(_, text, node, transform, visibility, layout)| (
                        &text.0,
                        node.size(),
                        transform.translation,
                        visibility,
                        layout.size
                    ))
                    .collect::<Vec<_>>()
            );
        }
    }
    app.world_mut().despawn(root);
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .all(|name| !name.as_str().starts_with("WaveformTime"))
    );
}

#[test]
fn current_frame_glyph_growth_cannot_escape_the_label_guard() {
    let (mut app, root, _) = fixture();
    let axis = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "WaveformTimeAxis").then_some(entity))
        .unwrap();
    for (second, scale, font_size) in [
        (99, 1.0, 12.0),
        (100, 1.5, 30.0),
        (999, 1.0, 12.0),
        (1000, 2.0, 24.0),
    ] {
        app.world_mut()
            .get_mut::<WaveformCursor>(root)
            .unwrap()
            .position = Duration::from_secs(second);
        app.world_mut().resource_mut::<UiScale>().0 = scale;
        let labels: Vec<_> = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .filter_map(|(entity, name)| (name.as_str() == "WaveformTimeLabel").then_some(entity))
            .collect();
        for label in labels {
            app.world_mut()
                .get_mut::<TextFont>(label)
                .unwrap()
                .font_size = bevy::text::FontSize::Px(font_size);
        }
        app.update();
        let node = app.world().get::<ComputedNode>(axis).unwrap();
        let center = app
            .world()
            .get::<UiGlobalTransform>(axis)
            .unwrap()
            .translation
            .x;
        let (left, right) = (center - node.size().x * 0.5, center + node.size().x * 0.5);
        let mut bounds = app
            .world_mut()
            .query::<(
                &Name,
                &bevy::text::TextLayoutInfo,
                &UiGlobalTransform,
                &InheritedVisibility,
            )>()
            .iter(app.world())
            .filter(|(name, _, _, visible)| name.as_str() == "WaveformTimeLabel" && visible.get())
            .map(|(_, layout, transform, _)| {
                (
                    transform.translation.x - layout.size.x * 0.5,
                    transform.translation.x + layout.size.x * 0.5,
                )
            })
            .collect::<Vec<_>>();
        bounds.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert!(
            bounds
                .iter()
                .all(|(a, b)| *a >= left - 0.5 && *b <= right + 0.5)
        );
        assert!(bounds.windows(2).all(|pair| pair[0].1 <= pair[1].0));
    }
}

#[test]
fn actual_layout_places_major_marks_at_the_grid_positions_during_resize() {
    let (mut app, root, _) = fixture();
    let grid = named_mesh(&mut app, "WaveformTimeGrid");
    for width in [500.0, 350.0, 700.0] {
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(width);
        for _ in 0..2 {
            app.update();
            let (plot, transform) = app
                .world_mut()
                .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<ViewportNode>>()
                .single(app.world())
                .unwrap();
            let plot_left = transform.translation.x - plot.size().x * 0.5;
            let plot_width = plot.size().x;
            let marks: Vec<_> = app
                .world_mut()
                .query::<(&Name, &Node, &ComputedNode, &UiGlobalTransform)>()
                .iter(app.world())
                .filter(|(name, node, _, _)| {
                    name.as_str() == "WaveformTimeMark"
                        && node.display != Display::None
                        && node.height == px(6)
                })
                .map(|(_, _, node, transform)| transform.translation.x - node.size().x * 0.5)
                .collect();
            for quad in positions(&app, &grid).as_chunks::<4>().0 {
                let fraction = (quad[0][0] + quad[1][0]) * 0.5 + 0.5;
                if !(0.01..0.99).contains(&fraction) {
                    continue;
                }
                let x = plot_left + fraction * plot_width;
                assert!(
                    marks.iter().any(|mark| (mark - x).abs() < 0.05),
                    "grid {x} vs marks {marks:?}"
                );
            }
        }
    }
}

#[test]
fn replay_pause_seek_resume_and_new_session_preserve_grid_and_reset_axis() {
    let (mut app, root, _) = fixture();
    Waveform::set_grid_visible_in_world(app.world_mut(), root, false).unwrap();
    let source = Arc::new(Source {
        replay: true,
        ..default()
    });
    Waveform::set_session_in_world(app.world_mut(), root, source.clone()).unwrap();
    app.update();
    assert_eq!(labels(&mut app).first().unwrap(), "0 s");
    assert!(!Waveform::grid_visible(app.world(), root).unwrap());
    assert!(
        !app.world()
            .get::<WaveformRuntime>(root)
            .unwrap()
            .is_running()
    );
    assert_eq!(source.reads.load(Ordering::Relaxed), 0);
    Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .seek(Duration::from_secs(35));
    app.update();
    assert_eq!(labels(&mut app), ["26 s", "28 s", "30 s", "32 s", "34 s"]);
    Waveform::set_running_in_world(app.world_mut(), root, false).unwrap();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(40);
    app.update();
    assert_eq!(labels(&mut app), ["26 s", "28 s", "30 s", "32 s", "34 s"]);
    Waveform::set_running_in_world(app.world_mut(), root, true).unwrap();
    app.update();
    assert_eq!(labels(&mut app).last().unwrap(), "40 s");
}

#[test]
fn grid_can_be_initialized_without_a_session_and_survives_replacement() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    let root = app
        .world_mut()
        .spawn_scene(bsn! { @Waveform { @show_grid: false } })
        .unwrap()
        .id();
    assert!(!Waveform::grid_visible(app.world(), root).unwrap());
    Waveform::set_session_in_world(app.world_mut(), root, Arc::new(Source::default())).unwrap();
    assert!(!Waveform::grid_visible(app.world(), root).unwrap());
    Waveform::clear_session_in_world(app.world_mut(), root).unwrap();
    assert!(!Waveform::grid_visible(app.world(), root).unwrap());
    app.world_mut().despawn(root);
    assert_eq!(
        Waveform::set_grid_visible_in_world(app.world_mut(), root, true)
            .unwrap_err()
            .severity(),
        bevy::ecs::error::Severity::Error
    );
}
