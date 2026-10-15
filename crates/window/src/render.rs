use bevy::{
    camera::{CameraOutputMode, NormalizedRenderTarget},
    prelude::*,
    render::{
        camera::SortedCameras,
        renderer::{
            FlushCommands, RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance,
            RenderQueue,
        },
        settings::{RenderCreation, WgpuSettings},
        sync_world::MainEntity,
        view::window::ExtractedWindow,
    },
};
use bevy_widgetry_internal_log::widgetry_error;
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, DeviceType, Dx12BackendOptions, Dx12SwapchainKind,
    Features, Instance, InstanceDescriptor, RequestAdapterOptions,
};

pub(crate) fn submit_window_commands(mut commands: FlushCommands) {
    commands.flush();
}

// For new windows without cameras, no_camera_clear_pass would write multiple swap chains in one command list.
// Delay initial presentation and acquire back buffers only after each camera is ready.
pub(crate) fn defer_initial_present_without_camera(
    mut windows: Query<(MainEntity, &mut ExtractedWindow)>,
    cameras: Res<SortedCameras>,
) {
    for (main, mut window) in &mut windows {
        let has_camera = cameras.0.iter().any(|camera| {
            matches!(camera.target, Some(NormalizedRenderTarget::Window(target)) if target.entity() == main)
                && matches!(camera.output_mode, CameraOutputMode::Write { .. })
        });
        if !has_camera {
            window.needs_initial_present = false;
        }
    }
}

// Bevy automatic initialization does not expose the DX12 presentation system parameter.
// Create the DxgiFromVisual resource explicitly so directly launched Apps do not need an external environment variable for transparent windows.
pub async fn transparent_render_creation() -> Result<RenderCreation> {
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
        .await
        .map_err(|error| {
            widgetry_error!(backend = "DX12", error = %error, "透明 renderer 的 adapter 创建失败");
            BevyError::error(error)
        })?;
    let adapter_info = adapter.get_info();
    let mut features = adapter.features() - Features::all_experimental_mask();
    if adapter_info.device_type == DeviceType::DiscreteGpu {
        features.remove(Features::MAPPABLE_PRIMARY_BUFFERS);
    }
    let (device, queue) = adapter
        .request_device(&DeviceDescriptor {
            label: Some("Widgetry"),
            required_features: features,
            required_limits: adapter.limits(),
            memory_hints: settings.memory_hints,
            ..default()
        })
        .await
        .map_err(|error| {
            widgetry_error!(backend = "DX12", error = %error, "透明 renderer 的 device 创建失败");
            BevyError::error(error)
        })?;
    Ok(RenderCreation::manual(
        RenderDevice::from(device),
        RenderQueue::new(queue),
        RenderAdapterInfo::new(adapter_info),
        RenderAdapter::new(adapter),
        RenderInstance::new(instance),
    ))
}
