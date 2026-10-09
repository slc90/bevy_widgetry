//! State：待初始化/可见/隐藏/销毁，Polyline/Envelope，稳定/改变 layout，读取成功/失败。
//! Stimuli：真实 BSN spawn、Node resize、cursor 推进、source failure、style mutation、root despawn。
//! Guards：source/config/style 必须合法，零 layout size 不绘制。
//! Invariants：单 viewport/camera/mesh、lane/value/palette 稳定、失败保留 mesh、资源与容量有界。
//! Couplings：实际 physical layout width 决定 density；palette/background 按当前 Disabled、覆盖与 Theme 解析，clear 恢复当前状态的 Theme。
//! root ownership 同时管理 entity 和 asset。
//! Coverage Map：headless.rs 负责数据提交。
//! 本文件负责它与实际 BSN/layout/mesh 的组合。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::ecs::schedule::{ScheduleLabel, SingleThreadedExecutor};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::*;
use bevy_widgetry_core::scene::WidgetrySceneCommandsExt;
use bevy_widgetry_test_utils::{
    ErrorCapture, LogCapture, add_ui_plugins, scene_app, spawn_ui_camera,
};
use bevy_widgetry_waveform::*;
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
struct Source(AtomicBool, AtomicUsize);

impl WaveformSource for Source {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError> {
        self.1.fetch_add(1, Ordering::Relaxed);
        for channel in out.channels_mut() {
            channel.extend(
                range
                    .clone()
                    .map(|sample| if sample % 2 == 0 { -1.0 } else { 1.0 }),
            );
        }
        if self.0.load(Ordering::Relaxed) {
            Err(WaveformReadError("unavailable".into()))
        } else {
            Ok(())
        }
    }
}

fn fixture(rate: u32, width: f32) -> (App, Entity, Arc<Source>) {
    let mut app = scene_app();
    add_ui_plugins(&mut app);
    app.add_plugins((TransformPlugin, WaveformRenderPlugin));
    app.set_error_handler(ErrorCapture::handler());
    for label in [Update.intern(), PostUpdate.intern()] {
        app.edit_schedule(label, |schedule| {
            schedule.set_executor(SingleThreadedExecutor::new());
        });
    }
    let camera = spawn_ui_camera(&mut app, UVec2::new(1000, 800), 1.0);
    let source = Arc::new(Source::default());
    let adapter: Arc<dyn WaveformSource> = source.clone();
    let root = app.world_mut().spawn_scene(bsn! { @Waveform { @config: {WaveformConfig { sample_rate: rate, visible_duration_ms: 1000, channel_ranges: vec![-1.0..=1.0; 4] }}, @source: {Some(adapter)} } template(move |_| Ok(UiTargetCamera(camera))) Node { width: px(width), height: px(120) } }).unwrap().id();
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = duration_from_frames(u64::from(rate), rate).unwrap();
    app.update();
    (app, root, source)
}

fn mesh(app: &mut App) -> (Entity, Handle<Mesh>) {
    let (entity, mesh) = app
        .world_mut()
        .query::<(Entity, &Mesh2d)>()
        .single(app.world())
        .unwrap();
    (entity, mesh.0.clone())
}

fn assert_waveform_colors(
    app: &App,
    mesh: &Handle<Mesh>,
    camera: Entity,
    palette: &[Color],
    background: Color,
) {
    let mesh = app.world().resource::<Assets<Mesh>>().get(mesh).unwrap();
    let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
    else {
        panic!("vertex colors required");
    };
    assert!(!colors.is_empty());
    assert_eq!(colors.len() % 4, 0);
    // fixture 的四个 channel 各有相同数量的 envelope vertices；检查全部 vertices，而非只比较颜色集合。
    for (channel, vertices) in colors.chunks_exact(colors.len() / 4).enumerate() {
        let expected = palette[channel % palette.len()].to_linear().to_f32_array();
        assert!(
            vertices.iter().all(|actual| *actual == expected),
            "channel {channel} palette mismatch"
        );
    }
    assert!(
        matches!(app.world().get::<Camera>(camera).unwrap().clear_color, ClearColorConfig::Custom(color) if color == background)
    );
}

#[test]
fn palette_overrides_theme_and_disabled_only_recolor_existing_geometry() {
    use bevy::ui::InteractionDisabled;
    use bevy_widgetry_theme::WidgetryThemeMode;
    let (mut app, root, source) = fixture(1000, 200.0);
    let (_, handle) = mesh(&mut app);
    let camera = app
        .world_mut()
        .query::<&ViewportNode>()
        .single(app.world())
        .unwrap()
        .camera
        .unwrap();
    let original = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .clone();
    let revision = app.world().get::<WaveformRuntime>(root).unwrap().revision();
    let reads = source.1.load(Ordering::Relaxed);
    let unchanged = |app: &mut App| {
        assert_eq!(mesh(app).1, handle);
        let current = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap();
        assert_eq!(
            current.attribute(Mesh::ATTRIBUTE_POSITION),
            original.attribute(Mesh::ATTRIBUTE_POSITION)
        );
        assert_eq!(current.indices(), original.indices());
        assert_eq!(
            app.world().get::<WaveformRuntime>(root).unwrap().revision(),
            revision
        );
        assert_eq!(source.1.load(Ordering::Relaxed), reads);
    };
    let mut colors = WidgetryWaveformColorOverrides::default();
    colors.normal.palette = Some(vec![Color::WHITE, Color::BLACK]);
    colors.normal.background = Some(Color::NONE);
    colors.disabled.palette = Some(vec![Color::srgb(1.0, 0.0, 0.0), Color::srgb(0.0, 1.0, 0.0)]);
    colors.disabled.background = Some(Color::srgb(0.1, 0.2, 0.3));
    for invalid in [
        vec![],
        vec![Color::WHITE],
        vec![Color::WHITE, Color::WHITE],
        vec![Color::WHITE, Color::BLACK, Color::WHITE],
    ] {
        let mut invalid_colors = colors.clone();
        invalid_colors.disabled.palette = Some(invalid);
        let logs = LogCapture::default();
        let error = logs
            .run(|| {
                WidgetryWaveformColorOverrides::set_in_world(app.world_mut(), root, invalid_colors)
            })
            .unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("Waveform palette"));
        assert_eq!(
            logs.records()
                .iter()
                .filter(|record| record.target == "bevy_widgetry"
                    && record.level == bevy::log::Level::ERROR)
                .count(),
            1
        );
        assert_eq!(
            WidgetryWaveformColorOverrides::get(app.world(), root).unwrap(),
            &WidgetryWaveformColorOverrides::default()
        );
    }
    WidgetryWaveformColorOverrides::set_in_world(app.world_mut(), root, colors.clone()).unwrap();
    for (mode, disabled) in [
        (WidgetryThemeMode::Dark, false),
        (WidgetryThemeMode::Dark, true),
        (WidgetryThemeMode::Light, true),
        (WidgetryThemeMode::Light, false),
    ] {
        if disabled {
            app.world_mut().entity_mut(root).insert(InteractionDisabled);
        } else {
            app.world_mut()
                .entity_mut(root)
                .remove::<InteractionDisabled>();
        }
        WidgetryThemeMode::set_in_world(app.world_mut(), mode).unwrap();
        app.update();
        let expected = if disabled {
            &colors.disabled
        } else {
            &colors.normal
        };
        assert_waveform_colors(
            &app,
            &handle,
            camera,
            expected.palette.as_ref().unwrap(),
            expected.background.unwrap(),
        );
        unchanged(&mut app);
    }
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    WidgetryWaveformColorOverrides::clear_in_world(app.world_mut(), root).unwrap();
    app.update();
    let theme = WidgetryThemeMode::Light.colors().waveform.disabled;
    assert_waveform_colors(&app, &handle, camera, theme.palette, theme.background);
    unchanged(&mut app);
    app.world_mut()
        .entity_mut(root)
        .remove::<InteractionDisabled>();
    app.update();
    let theme = WidgetryThemeMode::Light.colors().waveform.normal;
    assert_waveform_colors(&app, &handle, camera, theme.palette, theme.background);
    unchanged(&mut app);
    WidgetryThemeMode::set_in_world(app.world_mut(), WidgetryThemeMode::Dark).unwrap();
    app.update();
    let theme = WidgetryThemeMode::Dark.colors().waveform.normal;
    assert_waveform_colors(&app, &handle, camera, theme.palette, theme.background);
    unchanged(&mut app);
    // normal 覆盖不得成为 disabled 缺省叶的 fallback。
    colors.disabled = Default::default();
    WidgetryWaveformColorOverrides::set_in_world(app.world_mut(), root, colors).unwrap();
    app.world_mut().entity_mut(root).insert(InteractionDisabled);
    app.update();
    let theme = WidgetryThemeMode::Dark.colors().waveform.disabled;
    assert_waveform_colors(&app, &handle, camera, theme.palette, theme.background);
    unchanged(&mut app);
}

#[test]
fn bsn_layout_drives_single_renderer_and_fixed_lanes() {
    let (mut app, root, _) = fixture(1000, 200.0);
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(runtime.config().channel_ranges.len(), 4);
    assert!(
        matches!(&runtime.reduced_channels()[0], ReducedChannel::Envelope(spans) if spans.len() == 200)
    );
    assert_eq!(
        app.world().get::<WaveformOutputLength>(root).unwrap().0,
        200
    );
    let viewport = app
        .world_mut()
        .query::<&ViewportNode>()
        .single(app.world())
        .unwrap();
    let camera = viewport.camera.unwrap();
    assert!(app.world().get::<Camera2d>(camera).is_some());
    assert!(
        app.world()
            .get::<RenderTarget>(camera)
            .unwrap()
            .as_image()
            .is_some()
    );
    let (_, handle) = mesh(&mut app);
    let mesh = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap();
    let Some(VertexAttributeValues::Float32x3(positions)) =
        mesh.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("positions required");
    };
    assert_eq!(positions.len(), 200 * 4 * 4);
    assert_eq!(positions[0][1], 0.25);
    assert_eq!(positions[2][1], 0.5);
    let Some(VertexAttributeValues::Float32x4(colors)) = mesh.attribute(Mesh::ATTRIBUTE_COLOR)
    else {
        panic!("colors required");
    };
    assert_ne!(colors[0], colors[200 * 4]);
    assert_eq!(mesh.indices().unwrap().len(), 200 * 4 * 6);
}

#[test]
fn resize_and_read_failure_preserve_renderer_ownership() {
    let errors = ErrorCapture::default();
    errors.run(|| {
        let (mut app, root, source) = fixture(100, 200.0);
        let (entity, handle) = mesh(&mut app);
        assert!(matches!(&app.world().get::<WaveformRuntime>(root).unwrap().reduced_channels()[0], ReducedChannel::Polyline(points) if points.len() == 100));
        let before = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap().attribute(Mesh::ATTRIBUTE_POSITION).unwrap().clone();
        source.0.store(true, Ordering::Relaxed);
        app.world_mut().get_mut::<WaveformCursor>(root).unwrap().position = duration_from_frames(110, 100).unwrap();
        app.update();
        assert_eq!(app.world().resource::<Assets<Mesh>>().get(&handle).unwrap().attribute(Mesh::ATTRIBUTE_POSITION).unwrap(), &before);
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(50);
        app.update();
        assert_eq!(app.world().get::<WaveformRuntime>(root).unwrap().buffered_range(), 0..100);
        assert_eq!(app.world().get::<WaveformOutputLength>(root).unwrap().0, 50);
        source.0.store(false, Ordering::Relaxed);
        app.world_mut().get_mut::<Node>(root).unwrap().width = px(25);
        app.update();
        assert_eq!(mesh(&mut app), (entity, handle.clone()));
        assert_eq!(app.world().get::<WaveformOutputLength>(root).unwrap().0, 25);
        assert!(matches!(&app.world().get::<WaveformRuntime>(root).unwrap().reduced_channels()[0], ReducedChannel::Envelope(_)));
        let assets = app.world().resource::<Assets<Image>>();
        let image = assets.iter().next().unwrap().1;
        assert_eq!(image.size(), UVec2::new(25, 120));
    });
    assert_eq!(errors.take().len(), 2);
}

#[test]
fn sustained_render_and_despawn_keep_resources_bounded() {
    let (mut app, root, _) = fixture(1000, 200.0);
    let (entity, handle) = mesh(&mut app);
    let count = app.world().entities().count_spawned();
    let mut capacity = None;
    for frame in (1010..=4000).step_by(10) {
        app.world_mut()
            .get_mut::<WaveformCursor>(root)
            .unwrap()
            .position = duration_from_frames(frame, 1000).unwrap();
        app.update();
        assert_eq!(mesh(&mut app), (entity, handle.clone()));
        assert_eq!(app.world().entities().count_spawned(), count);
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 1);
        let mesh = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap();
        let Some(VertexAttributeValues::Float32x3(positions)) =
            mesh.attribute(Mesh::ATTRIBUTE_POSITION)
        else {
            panic!("positions required");
        };
        if frame > 1200 {
            assert_eq!(Some(positions.capacity()), capacity);
        } else if frame == 1200 {
            capacity = Some(positions.capacity());
        }
    }
    app.world_mut().despawn(root);
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 0);
    assert_eq!(app.world().resource::<Assets<ColorMaterial>>().len(), 0);
    assert_eq!(
        app.world_mut().query::<&Mesh2d>().iter(app.world()).count(),
        0
    );
    assert_eq!(
        app.world_mut()
            .query::<&Camera2d>()
            .iter(app.world())
            .count(),
        1
    );
}

#[test]
fn hidden_style_changes_apply_when_visibility_returns() {
    let (mut app, root, _) = fixture(1000, 200.0);
    let camera = app
        .world_mut()
        .query::<&ViewportNode>()
        .single(app.world())
        .unwrap()
        .camera
        .unwrap();
    let (_, handle) = mesh(&mut app);
    app.world_mut()
        .get_mut::<Visibility>(root)
        .unwrap()
        .set_if_neq(Visibility::Hidden);
    app.update();
    assert!(!app.world().get::<Camera>(camera).unwrap().is_active);
    let mut palette = bevy_widgetry_theme::WIDGETRY_DARK_THEME
        .waveform
        .normal
        .palette
        .to_vec();
    palette[0] = Color::WHITE;
    WidgetryWaveformColorOverrides::set_in_world(
        app.world_mut(),
        root,
        WidgetryWaveformColorOverrides {
            normal: WidgetryWaveformStateColorOverrides {
                palette: Some(palette),
                ..default()
            },
            ..default()
        },
    )
    .unwrap();
    app.update();
    app.update();
    app.world_mut()
        .get_mut::<Visibility>(root)
        .unwrap()
        .set_if_neq(Visibility::Inherited);
    app.update();
    assert!(app.world().get::<Camera>(camera).unwrap().is_active);
    let colors = app
        .world()
        .resource::<Assets<Mesh>>()
        .get(&handle)
        .unwrap()
        .attribute(Mesh::ATTRIBUTE_COLOR)
        .unwrap();
    assert!(matches!(colors, VertexAttributeValues::Float32x4(values) if values[0] == [1.0; 4]));
}

#[test]
fn multiple_views_have_isolated_render_layers() {
    let (mut app, _, source) = fixture(1000, 200.0);
    let adapter: Arc<dyn WaveformSource> = source;
    app.world_mut()
        .spawn_scene(
            bsn! { @Waveform { @source: {Some(adapter)} } Node { width: px(100), height: px(60) } },
        )
        .unwrap();
    app.update();
    let cameras: Vec<_> = app
        .world_mut()
        .query::<&ViewportNode>()
        .iter(app.world())
        .map(|viewport| viewport.camera.unwrap())
        .collect();
    assert_eq!(cameras.len(), 2);
    assert!(
        !app.world()
            .get::<RenderLayers>(cameras[0])
            .unwrap()
            .intersects(app.world().get::<RenderLayers>(cameras[1]).unwrap())
    );
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 2);
    assert_eq!(app.world().resource::<Assets<Image>>().len(), 2);
}

#[test]
fn scene_rejects_invalid_configuration_without_panicking() {
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| {
        let mut app = scene_app();
        app.add_plugins(WaveformRenderPlugin);
        app.set_error_handler(ErrorCapture::handler());
        app.world_mut().commands().spawn_scene_with_error_handler(bsn! { @Waveform });
        for invalid_style in [false, true] {
            let adapter: Arc<dyn WaveformSource> = Arc::new(Source::default());
            app.world_mut().commands().spawn_scene_with_error_handler(bsn! { @Waveform { @source: {Some(adapter)}, @config: {WaveformConfig { sample_rate: if invalid_style { 1000 } else { 0 }, visible_duration_ms: 1000, channel_ranges: vec![-1.0..=1.0] }}, @style: {WaveformStyle { line_width: if invalid_style { 0.0 } else { 1.0 } }} } });
        }
        app.world_mut().flush();
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
        assert_eq!(app.world().resource::<Assets<Image>>().len(), 0);
        assert_eq!(app.world_mut().query::<&Waveform>().iter(app.world()).count(), 0);
    }));
    let failures = errors.take();
    assert_eq!(failures.len(), 3);
    assert!(
        failures
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        3
    );
}

#[test]
fn plugin_preserves_existing_host_assets() {
    let mut app = scene_app();
    app.init_asset::<Mesh>().init_asset::<ColorMaterial>();
    let image = app
        .world_mut()
        .resource_mut::<Assets<Image>>()
        .add(Image::default());
    let mesh = app
        .world_mut()
        .resource_mut::<Assets<Mesh>>()
        .add(Mesh::from(Rectangle::new(3.0, 4.0)));
    let material = app
        .world_mut()
        .resource_mut::<Assets<ColorMaterial>>()
        .add(ColorMaterial::from(Color::WHITE));
    app.add_plugins(WaveformRenderPlugin);
    assert!(app.world().resource::<Assets<Image>>().contains(&image));
    assert!(app.world().resource::<Assets<Mesh>>().contains(&mesh));
    assert!(
        app.world()
            .resource::<Assets<ColorMaterial>>()
            .contains(&material)
    );
}

#[cfg(target_pointer_width = "64")]
#[test]
fn scene_preserves_allocation_failure_diagnostics() {
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            let mut app = scene_app();
            app.set_error_handler(ErrorCapture::handler());
            let config = WaveformConfig {
                sample_rate: 1_000_000_000,
                visible_duration_ms: u32::MAX,
                channel_ranges: vec![-1.0..=1.0],
            };
            assert!(config.capacity_frames().is_ok());
            let adapter: Arc<dyn WaveformSource> = Arc::new(Source::default());
            app.world_mut().commands().spawn_scene_with_error_handler(
                bsn! { @Waveform { @source: {Some(adapter)}, @config: config } },
            );
            app.world_mut().flush();
        })
    });
    let failures = errors.take();
    assert_eq!(failures.len(), 1);
    assert!(failures[0].to_string().contains("memory allocation failed"));
    assert_eq!(failures[0].severity(), bevy::ecs::error::Severity::Error);
    assert_eq!(
        logs.records()
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        1
    );
}

#[test]
fn empty_display_has_only_invisible_valid_triangles() {
    let (mut app, root, _) = fixture(1000, 200.0);
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = std::time::Duration::ZERO;
    app.update();
    let (_, handle) = mesh(&mut app);
    let asset = app.world().resource::<Assets<Mesh>>().get(&handle).unwrap();
    assert!(asset.count_vertices() >= 3);
    assert!(asset.indices().unwrap().len() >= 3);
    let Some(VertexAttributeValues::Float32x3(positions)) =
        asset.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        panic!("缺少 position attribute");
    };
    assert!(positions.iter().all(|position| *position == [0.0; 3]));
    let Some(VertexAttributeValues::Float32x4(colors)) = asset.attribute(Mesh::ATTRIBUTE_COLOR)
    else {
        panic!("缺少 color attribute");
    };
    assert!(colors.iter().all(|color| color[3] == 0.0));
    app.world_mut().despawn(root);
    app.update();
    assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
    assert_eq!(
        app.world_mut().query::<&Camera>().iter(app.world()).count(),
        1
    );
}
