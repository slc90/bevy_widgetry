#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use super::*;
use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    render::render_resource::{Extent3d, TextureDimension},
};
use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};

fn image() -> Image {
    Image::new_fill(
        Extent3d {
            width: 2,
            height: 2,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255; 4],
        TextureFormat::Rgba8UnormSrgb,
        default(),
    )
}

fn check(app: &App, handle: &Handle<Image>, state: &mut FailureState<FailureKey>) -> bool {
    usable(
        app.world().resource::<AssetServer>(),
        app.world().resource::<Assets<Image>>(),
        handle,
        UVec2::splat(2),
        state,
    )
}

#[test]
fn pixel_failures_keep_actual_fields_independent_assets_and_recover_only_when_valid() {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>();
    let first = app.world_mut().resource_mut::<Assets<Image>>().add(image());
    let second = app.world_mut().resource_mut::<Assets<Image>>().add(image());
    let mut a = FailureState::default();
    let mut b = FailureState::default();
    let logs = LogCapture::default();
    logs.run(|| {
        assert!(check(&app, &first, &mut a));
        assert!(logs.records().is_empty());
        for handle in [&first, &second] {
            app.world_mut().resource_mut::<Assets<Image>>().get_mut(handle).unwrap().data = Some(vec![0; 3]);
        }
        assert!(!check(&app, &first, &mut a));
        let origin = Arc::clone(a.active().unwrap().origin());
        assert!(!check(&app, &first, &mut a));
        assert!(Arc::ptr_eq(a.active().unwrap().origin(), &origin));
        assert!(!check(&app, &second, &mut b));
        assert_ne!(a.active().unwrap().origin().id(), b.active().unwrap().origin().id());
        let kind = a.active().unwrap().source().unwrap().downcast_ref::<CursorError>().unwrap();
        assert!(matches!(kind, CursorError::Pixels { pixels, .. } if pixels.expected == UVec2::splat(2) && pixels.actual == UVec2::splat(2) && pixels.bytes == Some(3) && pixels.format == TextureFormat::Rgba8UnormSrgb));
        assert!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled).is_empty());
        assert_eq!(logs.diagnostic_records(origin.id(), DiagnosticPhase::Source).len(), 1);
        let saved = app.world_mut().resource_mut::<Assets<Image>>().remove(first.id()).unwrap();
        assert!(!check(&app, &first, &mut a));
        assert!(Arc::ptr_eq(a.active().unwrap().origin(), &origin));
        app.world_mut().resource_mut::<Assets<Image>>().insert(first.id(), saved).unwrap();
        app.world_mut().resource_mut::<Assets<Image>>().insert(first.id(), image()).unwrap();
        assert!(check(&app, &first, &mut a));
        assert!(check(&app, &first, &mut a));
        assert_eq!(logs.records().iter().filter(|record| record.matches_field("phase", "recovered")).count(), 1);
        app.world_mut().resource_mut::<Assets<Image>>().get_mut(&first).unwrap().texture_descriptor.format = TextureFormat::Bgra8Unorm;
        assert!(!check(&app, &first, &mut a));
        assert_ne!(a.active().unwrap().origin().id(), origin.id());
        let previous = Arc::clone(a.active().unwrap().origin());
        app.world_mut().resource_mut::<Assets<Image>>().get_mut(&first).unwrap().texture_descriptor.size.width = 3;
        assert!(!check(&app, &first, &mut a));
        assert_ne!(a.active().unwrap().origin().id(), previous.id());
    });
}

#[derive(TypePath)]
struct ManagedLoader(WidgetryEcsError);
impl AssetLoader for ManagedLoader {
    type Asset = Image;
    type Settings = ();
    type Error = BevyError;
    async fn load(
        &self,
        _: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> Result<Image, BevyError> {
        Err(self.0.clone().into_bevy_error())
    }
    fn extensions(&self) -> &[&str] {
        &["curdiag"]
    }
}

#[test]
fn managed_loader_batch_keeps_all_original_diagnostics_and_external_source_is_retained() {
    use bevy::asset::io::{
        AssetSourceBuilder, AssetSourceId,
        memory::{Dir, MemoryAssetReader},
    };
    use std::{
        path::Path,
        time::{Duration, Instant},
    };
    let logs = LogCapture::default();
    logs.run(|| {
        let first = Arc::new(WidgetryFailure::new(
            std::io::Error::from_raw_os_error(1234),
            ErrorFrame::new("loader.first"),
        ));
        let second = Arc::new(WidgetryFailure::new(
            std::io::Error::from_raw_os_error(2345),
            ErrorFrame::new("loader.second"),
        ));
        let origins = [Arc::clone(first.origin()), Arc::clone(second.origin())];
        let mut managed = WidgetryEcsError::from_shared(first);
        managed.push_shared(second);
        let dir = Dir::default();
        dir.insert_asset_text(Path::new("test.curdiag"), "unused");
        let mut app = App::new();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || Box::new(MemoryAssetReader { root: dir.clone() })),
        );
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .register_asset_loader(ManagedLoader(managed));
        let handle: Handle<Image> = app.world().resource::<AssetServer>().load("test.curdiag");
        let deadline = Instant::now() + Duration::from_secs(5);
        let source = loop {
            app.update();
            if let LoadState::Failed(error) = app
                .world()
                .resource::<AssetServer>()
                .load_state(handle.id())
            {
                break error;
            }
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        };
        let failure = load_failure(&handle, source);
        assert_eq!(failure.diagnostics().count(), 2);
        let mut state = FailureState::default();
        state.failure_batch(
            FailureKey::Pixels(Pixels {
                asset: handle.id(),
                expected: UVec2::splat(2),
                actual: UVec2::ONE,
                format: TextureFormat::Rgba8Unorm,
                bytes: None,
            }),
            || failure,
        );
        for (diagnostic, origin) in state.diagnostics().zip(origins) {
            assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
            assert!(
                diagnostic
                    .contexts()
                    .iter()
                    .any(|frame| frame.operation() == "cursor.image")
            );
            let mut source: &dyn Error = diagnostic;
            while source.downcast_ref::<std::io::Error>().is_none() {
                source = source.source().unwrap();
            }
            assert!(
                source
                    .downcast_ref::<std::io::Error>()
                    .unwrap()
                    .raw_os_error()
                    .is_some()
            );
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
        }
        state.failure_batch(
            FailureKey::Pixels(Pixels {
                asset: handle.id(),
                expected: UVec2::splat(2),
                actual: UVec2::ONE,
                format: TextureFormat::Rgba8Unorm,
                bytes: None,
            }),
            || panic!("same key must not recreate the batch"),
        );
        handled(&state, Entity::PLACEHOLDER, "system cursor=Pointer");
        handled(&state, Entity::PLACEHOLDER, "system cursor=Pointer");
        for diagnostic in state.diagnostics() {
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .len(),
                1
            );
        }
        let ids: Vec<_> = state
            .diagnostics()
            .map(|diagnostic| diagnostic.origin().id().clone())
            .collect();
        assert!(state.recover(|| ErrorFrame::new("cursor.recovered")));
        assert!(!state.recover(|| panic!("idle recovery must not construct context")));
        for id in ids {
            assert_eq!(
                logs.records()
                    .iter()
                    .filter(|record| record.matches_field("error_id", &id.to_string())
                        && record.matches_field("phase", "recovered"))
                    .count(),
                1
            );
        }
    });
}
