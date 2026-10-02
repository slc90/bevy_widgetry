// 测试通过公开 BSN、ECS 与 asset 观测验证 contract，允许测试断言与 unwrap。
#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

//! State：待初始化/可见/隐藏/销毁，Polyline/Envelope，稳定/改变 layout，读取成功/失败。
//! Stimuli：真实 BSN spawn、Node resize、cursor 推进、source failure、style mutation、root despawn。
//! Guards：source/config/style 必须合法，零 layout size 不绘制。
//! Invariants：单 viewport/camera/mesh、lane/value/palette 稳定、失败保留 mesh、资源与容量有界。
//! Couplings：实际 physical layout width 决定 density；root ownership 同时管理实体和 asset。
//! Coverage Map：headless.rs 负责数据提交；本文件负责它与实际 BSN/layout/mesh 的组合。

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
use std::sync::atomic::{AtomicBool, Ordering};

/// 交替极值帮助观察完整 lane range；失败故意写入 partial staging。
#[derive(Default)]
struct Source(AtomicBool);

impl WaveformSource for Source {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError> {
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

/// 真实 UI layout 使用共享 helper；不注入 ComputedNode 或 geometry 结果。
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

/// 从真实 Mesh2d entity 读取合并 Mesh，不扩大生产内部 visibility。
fn mesh(app: &mut App) -> (Entity, Handle<Mesh>) {
    let (entity, mesh) = app
        .world_mut()
        .query::<(Entity, &Mesh2d)>()
        .single(app.world())
        .unwrap();
    (entity, mesh.0.clone())
}

// @Waveform 首帧建立唯一 renderer，并以实际 layout width 决定 Envelope，lane 极值和颜色正确。
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

// 低密度产生 Polyline triangle，resize 改变密度但 entity/handle 不重建，失败 cursor 不覆盖旧 mesh。
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

// 长期推进/wrap 及 root 销毁都不残留 owned camera/mesh/target，稳定负载复用 Vec capacity。
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

// 隐藏期间 style 改变不能丢失；恢复显示时仍重建颜色，并关闭隐藏实例的离屏 camera。
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
    app.world_mut()
        .get_mut::<WaveformStyle>(root)
        .unwrap()
        .palette[0] = Color::WHITE;
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

// 两个真实 BSN 实例的 camera layer 不相交，仍各只有一个共享多 channel 的 mesh/target。
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

// source、config 和 style 的非法 Scene 输入从真正 template 入口返回 Error severity 并记录原因。
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
            app.world_mut().commands().spawn_scene_with_error_handler(bsn! { @Waveform { @source: {Some(adapter)}, @config: {WaveformConfig { sample_rate: if invalid_style { 1000 } else { 0 }, visible_duration_ms: 1000, channel_ranges: vec![-1.0..=1.0] }}, @style: {WaveformStyle { palette: if invalid_style { vec![] } else { WaveformStyle::default().palette }, ..default() }} } });
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

// 宿主先注册的 asset identity 与 payload 在 renderer plugin 安装后仍然可用。
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

// 合法但远超 64-bit 地址空间可用内存的规格经 try_reserve 拒绝，Scene 保留 allocation 原因。
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

// 零输入和 rewind 到零依然清空画面；mesh 的透明零面积 placeholder 保证 GPU allocation 非零。
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
