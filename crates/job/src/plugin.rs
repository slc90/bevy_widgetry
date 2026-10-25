use crate::JobManager;
use bevy::{
    prelude::*,
    winit::{EventLoopProxyWrapper, WinitUserEvent},
};
use bevy_widgetry_internal_log::widgetry_info;
use std::sync::Arc;

pub struct JobManagerPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JobManagerSystems {
    BeginFrame,
    EndFrame,
}

#[derive(Resource, Default)]
struct FrameWake {
    seen: usize,
}

impl Plugin for JobManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JobManager>()
            .init_resource::<FrameWake>()
            .add_systems(First, begin_frame.in_set(JobManagerSystems::BeginFrame))
            .add_systems(Last, end_frame.in_set(JobManagerSystems::EndFrame));
        widgetry_info!("JobManagerPlugin 注册完成");
    }
}

fn begin_frame(
    manager: Res<JobManager>,
    proxy: Option<Res<EventLoopProxyWrapper>>,
    mut linked: Local<bool>,
    mut frame: ResMut<FrameWake>,
) -> Result {
    let wake = manager.wake();
    match proxy {
        Some(proxy) if !*linked || proxy.is_changed() => {
            let proxy = (*proxy).clone();
            wake.connect(Some(Arc::new(move || {
                proxy
                    .send_event(WinitUserEvent::WakeUp)
                    .map_err(|error| error.to_string())
            })));
            *linked = true;
        }
        None if *linked => {
            wake.connect(None);
            *linked = false;
        }
        _ => {}
    }
    manager.maintain();
    frame.seen = wake.revision();
    if let Some(error) = wake.take_error() {
        return Err(BevyError::error(error));
    }
    Ok(())
}

fn end_frame(manager: Res<JobManager>, frame: Res<FrameWake>) -> Result {
    let wake = manager.wake();
    wake.finish_update(frame.seen);
    if let Some(error) = wake.take_error() {
        return Err(BevyError::error(error));
    }
    Ok(())
}
