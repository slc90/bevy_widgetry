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

// DX12 限制单次提交包含的 swap chain 数量。
// 多 window Gallery 按 camera 分别 flush，避免组合提交超出 backend 限制。
pub(crate) struct GalleryRenderPlugin;

fn submit_window_commands(mut commands: FlushCommands) {
    commands.flush();
}

// 新 window 没有 camera 时，no_camera_clear_pass 会把多个 swap chain 写入同一 command list。
// 延后初始 present，等各自 camera 准备好再获取 back buffer。
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

// Bevy 自动初始化未暴露 DX12 presentation system 参数。
// 手动创建 DxgiFromVisual 资源，避免直接启动 Gallery exe 时透明 window 依赖外部环境变量。
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
                    // Core2d 不自动插入 ApplyDeferred。
                    // 先应用 deferred command buffer 再 flush，避免该 camera 的绘制 command 留到多 window 共用的提交中。
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
