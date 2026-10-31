#![allow(clippy::disallowed_macros)]

use super::*;
use bevy::{
    asset::{
        AssetLoadError, LoadState,
        io::{
            AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
            ReaderNotSeekableError, SeekableReader,
            memory::{Dir, MemoryAssetReader},
        },
    },
    ecs::schedule::SingleThreadedExecutor,
    tasks::futures_lite::AsyncRead,
};
use bevy_widgetry_test_utils::{
    DiagnosticPhase, ErrorCapture, LogCapture, advance_until, scene_app,
};
use std::{
    io,
    path::Path,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

struct BrokenReader;

impl AsyncRead for BrokenReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Err(io::Error::from_raw_os_error(1234)))
    }
}

impl Reader for BrokenReader {
    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Err(ReaderNotSeekableError)
    }
}

struct Source(MemoryAssetReader);

impl AssetReader for Source {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        let reader: Box<dyn Reader> = if path == Path::new("read.svg") {
            Box::new(BrokenReader)
        } else {
            Box::new(self.0.read(path).await?)
        };
        Ok(reader)
    }
    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        self.0.read_meta(path).await
    }
    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.0.read_directory(path).await
    }
    async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> {
        self.0.is_directory(path).await
    }
}

fn icon_app() -> App {
    let mut app = scene_app();
    app.set_error_handler(ErrorCapture::handler());
    app.add_plugins(WidgetryIconPlugin);
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    app
}

fn ready_icon(app: &mut App, oversized: bool) -> Entity {
    let dimension = if oversized { "4294967295" } else { "16" };
    let tree = resvg::usvg::Tree::from_str(
        &format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{dimension}" height="{dimension}"/>"#
        ),
        &resvg::usvg::Options::default(),
    )
    .unwrap();
    let handle = app
        .world_mut()
        .resource_mut::<Assets<svg::SvgAsset>>()
        .add(svg::SvgAsset::from_tree(tree));
    app.world_mut()
        .spawn_scene(bsn! { WidgetryIcon { svg: {handle} } })
        .unwrap()
        .id()
}

#[test]
fn raster_batch_keeps_every_owner_and_reuses_each_origin_until_recovery() {
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| {
        let mut app = icon_app();
        let owners = [ready_icon(&mut app, true), ready_icon(&mut app, true)];
        app.update();
        app.update();
        let records = errors.take_records();
        assert_eq!(records.len(), 2);
        let origins: Vec<_> = records.iter().map(|record| {
            let diagnostics: Vec<_> = record.error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            diagnostics.iter().map(|diagnostic| {
                let failure = diagnostic.source().unwrap().downcast_ref::<IconError>().unwrap();
                assert!(matches!(failure, IconError::Rasterize { entity, width: u32::MAX, height: u32::MAX, scale: 1.0, requested_size: None, .. } if owners.contains(entity)));
                Arc::clone(diagnostic.origin())
            }).collect::<Vec<_>>()
        }).collect();
        for (first, repeated) in origins[0].iter().zip(&origins[1]) {
            assert!(Arc::ptr_eq(first, repeated));
            assert_eq!(logs.diagnostic_records(first.id(), DiagnosticPhase::Source).len(), 1);
            assert!(logs.diagnostic_records(first.id(), DiagnosticPhase::Handled).is_empty());
        }
        assert_ne!(origins[0][0].id(), origins[0][1].id());
        app.world_mut().get_mut::<WidgetryIcon>(owners[0]).unwrap().max_size = Some(UVec2::splat(16));
        app.update();
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 1);
        app.world_mut().entity_mut(owners[0]).remove::<IconMaterialized>();
        app.world_mut().get_mut::<WidgetryIcon>(owners[0]).unwrap().max_size = None;
        app.update();
        let new_origin = app.world().get::<IconRasterState>(owners[0]).unwrap().failure.active().unwrap().origin();
        assert!(origins[0].iter().all(|old| old.id() != new_origin.id()));
    }));
}

#[test]
fn missing_image_nodes_cross_real_system_boundaries_and_recover_once() {
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| errors.run(|| {
        let mut app = icon_app();
        let owners = [ready_icon(&mut app, false), ready_icon(&mut app, false)];
        app.update();
        let children: Vec<_> = owners.iter().map(|owner| app.world().get::<IconMaterialized>(*owner).unwrap().image_entity).collect();
        for child in &children { app.world_mut().entity_mut(*child).remove::<ImageNode>(); }
        app.update();
        app.update();
        let records = errors.take_records();
        assert_eq!(records.len(), 2);
        let mut first_ids = Vec::new();
        for record in records {
            assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
            let diagnostics: Vec<_> = record.error.downcast_ref::<WidgetryEcsError>().unwrap().diagnostics().collect();
            assert_eq!(diagnostics.len(), 2);
            for diagnostic in diagnostics {
                assert!(matches!(diagnostic.source().unwrap().downcast_ref::<IconError>().unwrap(), IconError::MissingImageNode { entity, image_entity } if owners.contains(entity) && children.contains(image_entity)));
                first_ids.push(diagnostic.origin().id().clone());
            }
        }
        assert_eq!(&first_ids[..2], &first_ids[2..]);
        for child in &children { app.world_mut().entity_mut(*child).insert(ImageNode::default()); }
        app.update();
        app.update();
        assert!(errors.take_records().is_empty());
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 2);
    }));
}

#[test]
fn real_loader_failures_keep_source_identity_and_actual_fallback_logs() {
    if std::env::var_os("WIDGETRY_ICON_LOADER_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .env("WIDGETRY_ICON_LOADER_CHILD", "1")
            .args(["--exact", "icon::diagnostic_tests::real_loader_failures_keep_source_identity_and_actual_fallback_logs", "--nocapture"])
            .output().unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let logs = LogCapture::default();
    bevy::log::tracing::dispatcher::set_global_default(logs.dispatcher()).unwrap();
    let directory = Dir::default();
    for name in ["parse.svg", "again.svg"] {
        directory.insert_asset_text(Path::new(name), "invalid SVG");
    }
    directory.insert_asset_text(
        Path::new("good.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"/>"#,
    );
    let mut app = App::new();
    app.register_asset_source(
        AssetSourceId::Default,
        AssetSourceBuilder::new(move || {
            Box::new(Source(MemoryAssetReader {
                root: directory.clone(),
            }))
        }),
    );
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        bevy::scene::ScenePlugin,
        WidgetryIconPlugin,
    ))
    .init_asset::<Image>();
    app.edit_schedule(PostUpdate, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    let mut ids = Vec::new();
    for path in ["read.svg", "parse.svg", "missing.svg"] {
        let owner =
            crate::scene::spawn_scene(app.world_mut(), bsn! { @WidgetryIcon { @path: {path} } })
                .unwrap();
        let handle = app.world().get::<WidgetryIcon>(owner).unwrap().svg.clone();
        advance_until(&mut app, Duration::from_secs(5), path, |world| {
            world
                .resource::<AssetServer>()
                .load_state(handle.id())
                .is_failed()
        })
        .unwrap();
        app.update();
        let LoadState::Failed(load_error) = app
            .world()
            .resource::<AssetServer>()
            .load_state(handle.id())
        else {
            panic!("loader must fail");
        };
        let diagnostic = app
            .world()
            .get::<IconRasterState>(owner)
            .unwrap()
            .failure
            .active()
            .unwrap();
        let id = diagnostic.origin().id().clone();
        if let AssetLoadError::AssetLoaderError(loader) = load_error.as_ref() {
            assert_eq!(loader.error().severity(), bevy::ecs::error::Severity::Error);
            let original = loader
                .error()
                .downcast_ref::<WidgetryEcsError>()
                .unwrap()
                .diagnostics()
                .next()
                .unwrap();
            assert!(Arc::ptr_eq(original.origin(), diagnostic.origin()));
            let kind = original
                .source()
                .unwrap()
                .downcast_ref::<IconError>()
                .unwrap();
            match kind {
                IconError::Read {
                    path: original_path,
                    source,
                } => {
                    assert_eq!(path, "read.svg");
                    assert_eq!(original_path.as_ref(), &AssetPath::from(path));
                    assert_eq!(source.raw_os_error(), Some(1234));
                }
                IconError::Parse {
                    path: original_path,
                    ..
                } => {
                    assert_eq!(path, "parse.svg");
                    assert_eq!(original_path.as_ref(), &AssetPath::from(path));
                }
                _ => panic!("loader lost its domain kind"),
            }
        } else {
            let kind = diagnostic
                .source()
                .unwrap()
                .downcast_ref::<IconError>()
                .unwrap();
            assert!(
                matches!(kind, IconError::AssetLoad { source, .. } if Arc::ptr_eq(source, &load_error))
            );
        }
        app.update();
        app.update();
        assert_eq!(
            logs.diagnostic_records(&id, DiagnosticPhase::Source).len(),
            1
        );
        assert_eq!(
            logs.diagnostic_records(&id, DiagnosticPhase::Handled).len(),
            1
        );
        assert!(
            logs.diagnostic_records(&id, DiagnosticPhase::Unhandled)
                .is_empty()
        );
        assert_eq!(
            app.world()
                .get::<IconRasterState>(owner)
                .unwrap()
                .failure
                .active()
                .unwrap()
                .origin()
                .id(),
            &id
        );
        WidgetryIcon::set_svg_in_world(app.world_mut(), owner, "good.svg").unwrap();
        advance_until(
            &mut app,
            Duration::from_secs(5),
            "recovered image",
            |world| world.get::<IconMaterialized>(owner).is_some(),
        )
        .unwrap();
        assert_eq!(
            logs.records_for_error(&id)
                .iter()
                .filter(|record| record.matches_field("phase", "recovered"))
                .count(),
            1
        );
        WidgetryIcon::set_svg_in_world(app.world_mut(), owner, "again.svg").unwrap();
        advance_until(
            &mut app,
            Duration::from_secs(5),
            "independent failure",
            |world| {
                world
                    .get::<IconRasterState>(owner)
                    .unwrap()
                    .failure
                    .active()
                    .is_some()
            },
        )
        .unwrap();
        let again = app
            .world()
            .get::<IconRasterState>(owner)
            .unwrap()
            .failure
            .active()
            .unwrap()
            .origin()
            .id();
        assert_ne!(again, &id);
        ids.push(id);
        app.world_mut().despawn(owner);
    }
    assert!(ids.windows(2).all(|ids| ids[0] != ids[1]));
}
