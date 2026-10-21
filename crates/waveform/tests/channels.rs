#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::prelude::*;
use bevy_widgetry_scroll_area::{WidgetryScrollAreaContent, WidgetryScrollAreaViewport};
use bevy_widgetry_test_utils::{add_ui_plugins, scene_app, spawn_ui_camera};
use bevy_widgetry_waveform::*;
use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

struct Source(usize, AtomicUsize);
impl WaveformSource for Source {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            channel_count: self.0,
            channel_labels: (0..self.0).map(|index| format!("Sensor {index}")).collect(),
            sample_rate: 100,
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Live,
        })
    }
    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        self.1.fetch_add(1, Ordering::Relaxed);
        for channel in out.channels_mut() {
            channel.resize((request.range.end - request.range.start) as usize, 0.0);
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

fn fixture(channels: usize, count: WaveformChannelCount) -> (App, Entity, Arc<Source>) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((TransformPlugin, WaveformRenderPlugin));
    let camera = spawn_ui_camera(&mut app, UVec2::new(1000, 1000), 1.0);
    let source = Arc::new(Source(channels, AtomicUsize::new(0)));
    let adapter: Arc<dyn WaveformSource> = source.clone();
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @Waveform { @source: {Some(adapter)}, @channels_per_screen: {count} }
            template(move |_| Ok(UiTargetCamera(camera)))
            Node { width: px(600), height: px(672) }
        })
        .unwrap()
        .id();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = Duration::from_secs(1);
    for _ in 0..4 {
        app.update();
    }
    (app, root, source)
}

#[test]
fn actual_density_obeys_minimum_and_fewer_channels_fill_the_viewport() {
    for (channels, target, height) in [
        (6, WaveformChannelCount::Count(16), 100.0),
        (64, WaveformChannelCount::Count(8), 75.0),
        (64, WaveformChannelCount::Count(16), 37.5),
        (64, WaveformChannelCount::Count(32), 24.0),
        (64, WaveformChannelCount::All, 24.0),
    ] {
        let (mut app, root, _) = fixture(channels, target);
        let layout = Waveform::channel_layout(app.world(), root).unwrap();
        assert_eq!(layout.viewport_height, 600.0);
        assert!((layout.row_height - height).abs() < 0.001, "{layout:?}");
        assert_eq!(layout.content_height, height * channels as f32);
        let labels = app
            .world_mut()
            .query::<(&Name, &Text, &UiGlobalTransform)>()
            .iter(app.world())
            .filter(|(name, _, _)| name.as_str() == "WaveformChannelLabel")
            .map(|(_, text, transform)| (text.0.clone(), transform.translation.y))
            .collect::<Vec<_>>();
        assert_eq!(labels.len(), channels);
        assert_eq!(labels[0].0, "Sensor 0");
        assert!((labels[1].1 - labels[0].1 - height).abs() < 0.001);
    }
}

#[test]
fn fractional_scroll_matches_waveform_centers_and_keeps_top_anchor_on_density_and_resize() {
    let (mut app, root, source) = fixture(64, WaveformChannelCount::Count(8));
    let viewport = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaViewport>>()
        .single(app.world())
        .unwrap();
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .y = 187.5;
    app.update();
    let old = Waveform::channel_layout(app.world(), root).unwrap();
    assert_eq!(old.scroll_offset, 187.0);
    let reads = source.1.load(Ordering::Relaxed);
    let revision = app.world().get::<WaveformRuntime>(root).unwrap().revision();
    let buffered = app
        .world()
        .get::<WaveformRuntime>(root)
        .unwrap()
        .buffered_range();
    Waveform::set_channels_per_screen_in_world(
        app.world_mut(),
        root,
        WaveformChannelCount::Count(16),
    )
    .unwrap();
    for _ in 0..2 {
        app.update();
    }
    assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 93.75).abs() < 0.01);
    app.world_mut().get_mut::<Node>(root).unwrap().height = px(872.375);
    for _ in 0..2 {
        app.update();
    }
    assert!((app.world().get::<ScrollPosition>(viewport).unwrap().y - 125.05859).abs() < 0.01);
    assert_eq!(source.1.load(Ordering::Relaxed), reads);
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(runtime.revision(), revision);
    assert_eq!(runtime.buffered_range(), buffered);
    let plot = app
        .world_mut()
        .query_filtered::<(&ComputedNode, &UiGlobalTransform), With<ViewportNode>>()
        .single(app.world())
        .unwrap();
    let plot_top = plot.1.translation.y - plot.0.size().y * 0.5;
    let plot_height = plot.0.size().y;
    let plot_center = plot.1.translation.y;
    let mesh = app
        .world_mut()
        .query::<(&Name, &Mesh2d)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "WaveformMesh")
        .unwrap()
        .1
        .0
        .clone();
    let bevy::mesh::VertexAttributeValues::Float32x3(points) = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&mesh)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
    else {
        panic!("missing positions")
    };
    let centers: Vec<_> = points
        .as_chunks::<4>()
        .0
        .iter()
        .map(|quad| plot_center - quad.iter().map(|p| p[1]).sum::<f32>() / 4.0 * plot_height)
        .collect();
    let layout = Waveform::channel_layout(app.world(), root).unwrap();
    for (_, label, transform) in app
        .world_mut()
        .query::<(&Name, &Text, &UiGlobalTransform)>()
        .iter(app.world())
        .filter(|(name, _, _)| name.as_str() == "WaveformChannelLabel")
    {
        let index: usize = label.0.strip_prefix("Sensor ").unwrap().parse().unwrap();
        assert!(
            (transform.translation.y
                - (plot_top + (index as f32 + 0.5) * layout.row_height - layout.scroll_offset))
                .abs()
                < 0.01
        );
        if transform.translation.y > plot_top + 1.0
            && transform.translation.y < plot_top + plot_height - 1.0
        {
            assert!(
                centers
                    .iter()
                    .any(|center| (center - transform.translation.y).abs() < 0.01)
            );
        }
    }
    let content = app
        .world_mut()
        .query_filtered::<Entity, With<WidgetryScrollAreaContent>>()
        .single(app.world())
        .unwrap();
    assert!(app.world().get::<Children>(content).unwrap().len() == 64);
}

#[derive(Resource, Default)]
struct Notices(Vec<WaveformChannelCount>);

#[test]
fn count_api_validates_choices_commits_before_notifying_and_preserves_other_settings() {
    let mut app = scene_app();
    app.init_resource::<Notices>();
    app.add_observer(
        |event: On<WaveformChannelCountChanged>, mut notices: ResMut<Notices>| {
            notices.0.push(event.count);
        },
    );
    let root = app.world_mut().spawn_scene(bsn! {
        @Waveform { @channel_counts: {vec![WaveformChannelCount::All, WaveformChannelCount::Count(16)]} }
    }).unwrap().id();
    assert!(
        Waveform::set_channels_per_screen_in_world(
            app.world_mut(),
            root,
            WaveformChannelCount::Count(16)
        )
        .unwrap()
    );
    assert_eq!(
        Waveform::channels_per_screen(app.world(), root).unwrap(),
        WaveformChannelCount::Count(16)
    );
    assert!(
        !Waveform::set_channels_per_screen_in_world(
            app.world_mut(),
            root,
            WaveformChannelCount::Count(16)
        )
        .unwrap()
    );
    for count in [
        WaveformChannelCount::Count(0),
        WaveformChannelCount::Count(8),
    ] {
        let error =
            Waveform::set_channels_per_screen_in_world(app.world_mut(), root, count).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert_eq!(
            Waveform::channels_per_screen(app.world(), root).unwrap(),
            WaveformChannelCount::Count(16)
        );
    }
    assert_eq!(
        app.world().resource::<Notices>().0,
        vec![WaveformChannelCount::Count(16)]
    );
    Waveform::set_channels_per_screen(
        &mut app.world_mut().commands(),
        root,
        WaveformChannelCount::All,
    );
    app.world_mut().flush();
    assert_eq!(
        Waveform::channels_per_screen(app.world(), root).unwrap(),
        WaveformChannelCount::All
    );
    app.world_mut().despawn(root);
    assert!(
        Waveform::set_channels_per_screen_in_world(
            app.world_mut(),
            root,
            WaveformChannelCount::All
        )
        .is_err()
    );
}

#[test]
fn independent_views_keep_scroll_offsets_and_narrow_roots_do_not_create_negative_geometry() {
    let (mut app, root, source) = fixture(64, WaveformChannelCount::Count(8));
    let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
    let adapter: Arc<dyn WaveformSource> = source.clone();
    let second = app.world_mut().spawn_scene(bsn! {
        @Waveform { @source: {Some(adapter)}, @channels_per_screen: {WaveformChannelCount::Count(16)} }
        template(move |_| Ok(UiTargetCamera(camera)))
        Node { width: px(600), height: px(672) }
    }).unwrap().id();
    for _ in 0..3 {
        app.update();
    }
    let first_viewport = Waveform::channel_scroll_viewport(app.world(), root).unwrap();
    let second_viewport = Waveform::channel_scroll_viewport(app.world(), second).unwrap();
    app.world_mut()
        .get_mut::<ScrollPosition>(first_viewport)
        .unwrap()
        .y = 187.5;
    app.world_mut()
        .get_mut::<ScrollPosition>(second_viewport)
        .unwrap()
        .y = 375.5;
    app.update();
    assert_eq!(
        Waveform::channel_layout(app.world(), root)
            .unwrap()
            .scroll_offset,
        187.0
    );
    assert_eq!(
        Waveform::channel_layout(app.world(), second)
            .unwrap()
            .scroll_offset,
        375.0
    );
    Waveform::set_channels_per_screen_in_world(
        app.world_mut(),
        root,
        WaveformChannelCount::Count(16),
    )
    .unwrap();
    app.update();
    assert_eq!(
        Waveform::channel_layout(app.world(), second)
            .unwrap()
            .scroll_offset,
        375.0
    );
    for width in [5.0, 30.0, 120.0, 150.0] {
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(width);
        app.update();
        assert!(
            app.world_mut()
                .query::<&ComputedNode>()
                .iter(app.world())
                .all(|node| node.size().cmpge(Vec2::ZERO).all())
        );
    }
}

#[test]
fn session_replacement_and_empty_state_keep_density_and_clamp_scroll_at_the_end() {
    let (mut app, root, _) = fixture(64, WaveformChannelCount::Count(8));
    let viewport = Waveform::channel_scroll_viewport(app.world(), root).unwrap();
    app.world_mut()
        .get_mut::<ScrollPosition>(viewport)
        .unwrap()
        .y = 4200.0;
    app.update();
    let layout = Waveform::channel_layout(app.world(), root).unwrap();
    assert_eq!(layout.scroll_offset, 4200.0);
    assert_eq!(layout.content_height - layout.viewport_height, 4200.0);
    Waveform::set_session_in_world(
        app.world_mut(),
        root,
        Arc::new(Source(6, AtomicUsize::new(0))),
    )
    .unwrap();
    for _ in 0..2 {
        app.update();
    }
    let layout = Waveform::channel_layout(app.world(), root).unwrap();
    assert_eq!(layout.row_height, 100.0);
    assert_eq!(layout.scroll_offset, 0.0);
    assert_eq!(
        Waveform::channels_per_screen(app.world(), root).unwrap(),
        WaveformChannelCount::Count(8)
    );
    let labels: Vec<_> = app
        .world_mut()
        .query::<(&Name, &Text)>()
        .iter(app.world())
        .filter(|(name, _)| name.as_str() == "WaveformChannelLabel")
        .map(|(_, text)| text.0.clone())
        .collect();
    assert_eq!(labels.len(), 6);
    assert!(labels.iter().all(|label| {
        label
            .strip_prefix("Sensor ")
            .unwrap()
            .parse::<usize>()
            .unwrap()
            < 6
    }));
    Waveform::clear_session_in_world(app.world_mut(), root).unwrap();
    app.update();
    assert_eq!(
        Waveform::channel_layout(app.world(), root)
            .unwrap()
            .row_height,
        0.0
    );
    assert_eq!(
        app.world_mut()
            .query::<(&Name, &Text)>()
            .iter(app.world())
            .filter(|(name, _)| name.as_str() == "WaveformChannelLabel")
            .count(),
        0
    );
}
