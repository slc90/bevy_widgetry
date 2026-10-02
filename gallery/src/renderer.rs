use bevy::{
    camera::{CameraOutputMode, NormalizedRenderTarget},
    core_pipeline::{Core2d, upscaling::upscaling},
    prelude::*,
    render::{
        Render, RenderApp, RenderSystems,
        camera::SortedCameras,
        renderer::{
            FlushCommands, RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance,
            RenderQueue, WgpuWrapper,
        },
        settings::{RenderCreation, WgpuSettings},
        view::window::{ExtractedWindows, prepare_windows},
    },
};
use std::sync::Arc;
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, DeviceType, Dx12BackendOptions, Dx12SwapchainKind,
    Features, Instance, InstanceDescriptor, RequestAdapterOptions,
};

/// 为 Gallery 的每个 2D window 单独提交绘制，避免 DX12 单次提交的 swap chain 数量限制。
pub(crate) struct GalleryRenderPlugin;

/// 每个 camera 的最后一个绘制 system 完成后提交，保持原有 GPU 命令顺序。
fn submit_window_commands(mut commands: FlushCommands) {
    commands.flush();
}

/// Windows 不需要 Wayland 的初始空白 present；等待 camera 就绪后再获取 back buffer。
/// 否则 Bevy 的 no_camera_clear_pass 会把多个新 window 的 swap chain 写入同一个 command list。
fn defer_initial_present_without_camera(
    mut windows: ResMut<ExtractedWindows>,
    cameras: Res<SortedCameras>,
) {
    for window in windows.values_mut() {
        let has_camera = cameras.0.iter().any(|camera| {
            matches!(camera.target, Some(NormalizedRenderTarget::Window(target)) if target.entity() == window.entity)
                && matches!(camera.output_mode, CameraOutputMode::Write { .. })
        });
        if !has_camera {
            window.needs_initial_present = false;
        }
    }
}

/// 为 Gallery 明确选择支持透明的 DX12 swap chain，使直接启动 exe 也不依赖环境变量。
/// Bevy 的自动初始化未暴露 presentation system 参数，因此通过其手动初始化入口交付同一组 GPU 资源。
pub(crate) async fn transparent_renderer() -> Result<RenderCreation> {
    let settings = WgpuSettings::default();
    let instance = Instance::new(InstanceDescriptor {
        backends: Backends::DX12,
        flags: settings.instance_flags,
        memory_budget_thresholds: settings.instance_memory_budget_thresholds,
        backend_options: BackendOptions {
            dx12: Dx12BackendOptions {
                shader_compiler: settings.dx12_shader_compiler,
                presentation_system: Dx12SwapchainKind::DxgiFromVisual,
                ..default()
            },
            ..default()
        },
        display: None,
    });
    let adapter = instance
        .request_adapter(&RequestAdapterOptions {
            power_preference: settings.power_preference,
            ..default()
        })
        .await?;
    let adapter_info = adapter.get_info();
    // 延续 Bevy 默认的设备能力选择，但不启用要求 unsafe 授权的实验能力。
    let mut features = adapter.features() - Features::all_experimental_mask();
    if adapter_info.device_type == DeviceType::DiscreteGpu {
        features.remove(Features::MAPPABLE_PRIMARY_BUFFERS);
    }
    let (device, queue) = adapter
        .request_device(&DeviceDescriptor {
            label: Some("Widget Gallery"),
            required_features: features,
            required_limits: adapter.limits(),
            memory_hints: settings.memory_hints,
            ..default()
        })
        .await?;
    Ok(RenderCreation::manual(
        RenderDevice::from(device),
        RenderQueue(Arc::new(WgpuWrapper::new(queue))),
        RenderAdapterInfo(WgpuWrapper::new(adapter_info)),
        RenderAdapter(Arc::new(WgpuWrapper::new(adapter))),
        RenderInstance(Arc::new(WgpuWrapper::new(instance))),
    ))
}

impl Plugin for GalleryRenderPlugin {
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app
                .add_systems(
                    Core2d,
                    // Core2d 不自动插入 ApplyDeferred；先收集 RenderContext 的 deferred command buffer。
                    (ApplyDeferred, submit_window_commands)
                        .chain()
                        .after(upscaling),
                )
                .add_systems(
                    Render,
                    defer_initial_present_without_camera
                        .in_set(RenderSystems::PrepareViews)
                        .before(prepare_windows),
                );
        }
    }
}
