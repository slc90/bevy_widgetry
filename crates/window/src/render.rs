use crate::{WindowError, WindowFailure};
use bevy::{
    prelude::*,
    render::{
        renderer::{RenderAdapter, RenderAdapterInfo, RenderDevice, RenderInstance, RenderQueue},
        settings::{RenderCreation, WgpuSettings},
    },
};
use bevy_widgetry_diagnostics::ErrorFrame;
use wgpu::{
    BackendOptions, Backends, DeviceDescriptor, DeviceType, Dx12BackendOptions, Dx12SwapchainKind,
    Features, Instance, InstanceDescriptor, RequestAdapterOptions,
};

// Bevy automatic initialization does not expose the DX12 presentation system parameter.
// Create the DxgiFromVisual resource explicitly so directly launched Apps do not need an external environment variable for transparent windows.
pub async fn transparent_render_creation() -> Result<RenderCreation, WindowFailure> {
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
        .map_err(adapter_failure)?;
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
        .map_err(|error| device_failure(error, adapter_info.clone()))?;
    Ok(RenderCreation::manual(
        RenderDevice::from(device),
        RenderQueue::new(queue),
        RenderAdapterInfo::new(adapter_info),
        RenderAdapter::new(adapter),
        RenderInstance::new(instance),
    ))
}

#[track_caller]
fn adapter_failure(source: wgpu::RequestAdapterError) -> WindowFailure {
    WindowFailure::new(
        WindowError::Adapter {
            backend: wgpu::Backend::Dx12,
            source,
        },
        ErrorFrame::new("window.render.adapter").field("backend", "DX12"),
    )
}

#[track_caller]
fn device_failure(source: wgpu::RequestDeviceError, adapter: wgpu::AdapterInfo) -> WindowFailure {
    let frame = ErrorFrame::new("window.render.device")
        .field("backend", "DX12")
        .field("adapter", format_args!("{adapter:?}"));
    WindowFailure::new(
        WindowError::Device {
            adapter: Box::new(adapter),
            source,
        },
        frame,
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{DiagnosticPhase, LogCapture};
    use std::error::Error;

    #[test]
    fn adapter_failure_preserves_wgpu_source_backend_and_source_identity() {
        let logs = LogCapture::default();
        let error = logs.run(|| adapter_failure(wgpu::RequestAdapterError::EnvNotSet));
        assert!(matches!(
            error.kind(),
            WindowError::Adapter {
                backend: wgpu::Backend::Dx12,
                source: wgpu::RequestAdapterError::EnvNotSet
            }
        ));
        assert!(
            error
                .source()
                .unwrap()
                .source()
                .unwrap()
                .downcast_ref::<wgpu::RequestAdapterError>()
                .is_some()
        );
        assert_eq!(
            logs.diagnostic_records(error.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        let origin = error.origin().clone();
        let native = error.into_bevy_error();
        let diagnostic = native
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(&origin, diagnostic.origin()));
        assert_eq!(native.severity(), bevy::ecs::error::Severity::Error);
    }

    #[test]
    fn device_failure_preserves_real_dx12_request_error_and_adapter_info() {
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::DX12,
            ..InstanceDescriptor::new_without_display_handle()
        });
        let adapter =
            bevy::tasks::block_on(instance.request_adapter(&RequestAdapterOptions::default()))
                .unwrap();
        let source = bevy::tasks::block_on(adapter.request_device(&DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_bind_groups: u32::MAX,
                ..default()
            },
            ..default()
        }))
        .unwrap_err();
        let info = adapter.get_info();
        let error = device_failure(source, info.clone());
        assert!(
            matches!(error.kind(), WindowError::Device { adapter, .. } if adapter.name == info.name && adapter.backend == wgpu::Backend::Dx12)
        );
        assert!(
            error
                .source()
                .unwrap()
                .source()
                .unwrap()
                .downcast_ref::<wgpu::RequestDeviceError>()
                .is_some()
        );
        assert!(
            error.contexts()[0]
                .fields()
                .iter()
                .any(|(key, value)| key == "adapter" && value.contains(&info.name))
        );
    }
}
