use super::SplashRevealMaterial;
use bevy::asset::AssetId;
use bevy::prelude::*;
use bevy::render::{
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    camera::ExtractedCamera,
    render_asset::RenderAssets,
    render_phase::ViewSortedRenderPhases,
    render_resource::PipelineCache,
    sync_world::MainEntity,
    texture::GpuImage,
    view::{ExtractedView, ViewTarget, window::ExtractedWindow},
};
use bevy::ui_render::{
    ImageNodeBindGroups, PreparedUiMaterial, TransparentUi, UiBatch, UiCameraView, UiMaterialBatch,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Instant;

#[derive(Resource, Clone)]
pub(super) struct SplashPresentation {
    pub(super) window: Entity,
    gallery_camera: Entity,
    camera: Entity,
    scene_root: Entity,
    overlay: Entity,
    material: AssetId<SplashRevealMaterial>,
    images: Vec<AssetId<Image>>,
    pub(super) source_ready: Arc<AtomicBool>,
    pub(super) presented: Arc<AtomicBool>,
    pub(super) present_count: Arc<AtomicU64>,
    pub(super) present_interval_ns: Arc<AtomicU64>,
}

impl SplashPresentation {
    pub(super) fn new(
        window: Entity,
        gallery_camera: Entity,
        camera: Entity,
        scene_root: Entity,
        overlay: Entity,
        material: AssetId<SplashRevealMaterial>,
        images: Vec<AssetId<Image>>,
    ) -> Self {
        Self {
            window,
            gallery_camera,
            camera,
            scene_root,
            overlay,
            material,
            images,
            source_ready: default(),
            presented: default(),
            present_count: default(),
            present_interval_ns: default(),
        }
    }
}

#[derive(Resource, Default)]
struct PendingSplash {
    source: Option<SplashPresentation>,
    acquired: bool,
    last_present: Option<Instant>,
}

pub(super) fn install(app: &mut App) -> Result {
    let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
        error!("Splash 首帧检查需要 RenderApp");
        return Err(BevyError::error("Splash 首帧检查需要 RenderApp"));
    };
    render_app
        .init_resource::<PendingSplash>()
        .add_systems(ExtractSchedule, extract)
        .add_systems(
            Render,
            acquired
                .after(RenderSystems::Prepare)
                .before(RenderSystems::Render),
        )
        .add_systems(
            Render,
            presented
                .after(RenderSystems::Render)
                .before(RenderSystems::Cleanup),
        );
    Ok(())
}

fn extract(source: Extract<Option<Res<SplashPresentation>>>, mut pending: ResMut<PendingSplash>) {
    pending.source = source.as_ref().map(|source| (*source).clone());
    pending.acquired = false;
}

fn acquired(windows: Query<(MainEntity, &ExtractedWindow)>, mut pending: ResMut<PendingSplash>) {
    pending.acquired = pending.source.as_ref().is_some_and(|source| {
        windows
            .iter()
            .any(|(main, window)| main == source.window && window.swap_chain_texture.is_some())
    });
}

fn presented(
    mut pending: ResMut<PendingSplash>,
    windows: Query<(MainEntity, &ExtractedWindow)>,
    cameras: Query<(&MainEntity, &ViewTarget, &UiCameraView, &ExtractedCamera)>,
    ui_views: Query<&ExtractedView>,
    phases: Res<ViewSortedRenderPhases<TransparentUi>>,
    pipelines: Res<PipelineCache>,
    batches: Query<&UiBatch>,
    bindings: Res<ImageNodeBindGroups>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    materials: Res<RenderAssets<PreparedUiMaterial<SplashRevealMaterial>>>,
    material_batches: Query<&UiMaterialBatch<SplashRevealMaterial>>,
) -> Result {
    let Some(source) = pending.source.clone() else {
        return Ok(());
    };
    let phase_for = |camera_entity| {
        cameras
            .iter()
            .find(|(main, _, _, _)| main.id() == camera_entity)
            .and_then(|(_, _, ui, _)| ui_views.get(ui.0).ok())
            .and_then(|view| phases.get(&view.retained_view_entity))
    };
    if source.images.iter().all(|id| gpu_images.get(*id).is_some())
        && phase_for(source.camera).is_some_and(|phase| {
            phase.items.values().any(|item| {
                item.entity.1.id() == source.scene_root
                    && !item.batch_range.is_empty()
                    && pipelines.get_render_pipeline(item.pipeline).is_some()
                    && batches.get(item.entity.0).is_ok_and(|batch| {
                        !batch.range.is_empty() && bindings.values.contains_key(&batch.image)
                    })
            })
        })
    {
        source.source_ready.store(true, Ordering::Release);
    }
    if !pending.acquired || !source.source_ready.load(Ordering::Acquire) {
        return Ok(());
    }
    let window_presented = windows
        .iter()
        .any(|(main, window)| main == source.window && window.swap_chain_texture.is_none());
    let target_presented = cameras
        .iter()
        .any(|(main, target, _, _)| main.id() == source.gallery_camera && target.needs_present());
    if window_presented
        && target_presented
        && materials.get(source.material).is_some()
        && phase_for(source.gallery_camera).is_some_and(|phase| {
            phase.items.values().any(|item| {
                item.entity.1.id() == source.overlay
                    && !item.batch_range.is_empty()
                    && pipelines.get_render_pipeline(item.pipeline).is_some()
                    && material_batches.get(item.entity.0).is_ok()
            })
        })
    {
        if !source.presented.load(Ordering::Acquire) {
            winsafe::DwmFlush().map_err(|error| {
                error!(error = %error, "等待 Splash 首帧呈现失败");
                BevyError::error(error)
            })?;
            source.presented.store(true, Ordering::Release);
        }
        let now = Instant::now();
        if let Some(previous) = pending.last_present.replace(now) {
            source.present_interval_ns.store(
                now.duration_since(previous).as_nanos() as u64,
                Ordering::Relaxed,
            );
        }
        source.present_count.fetch_add(1, Ordering::Release);
    }
    Ok(())
}
