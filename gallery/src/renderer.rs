use bevy::{
    prelude::*,
    render::{
        renderer::{
            RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue,
            WgpuWrapper,
        },
        settings::{RenderCreation, WgpuSettings},
    },
};
use std::sync::Arc;
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, DeviceType, Dx12BackendOptions, Dx12SwapchainKind,
    Features, Instance, InstanceDescriptor, RequestAdapterOptions,
};

/// 为 Gallery 明确选择支持透明的 DX12 交换链，使直接启动 exe 也不依赖环境变量。
/// Bevy 的自动初始化未暴露呈现系统参数，因此通过其手动初始化入口交付同一组 GPU 资源。
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
