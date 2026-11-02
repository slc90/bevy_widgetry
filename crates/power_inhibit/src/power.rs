use crate::{PowerError, PowerFailure, PowerFailures, PowerOperation, PowerPhase, RequestKind};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_internal_log::widgetry_info;
use std::io;

#[derive(Resource, Default)]
pub struct PowerInhibit {
    owner: Owner<native::WindowsApi>,
}

impl PowerInhibit {
    pub fn enable(&mut self) -> Result<(), PowerFailures> {
        self.owner.enable()
    }

    pub fn disable(&mut self) -> Result<(), PowerFailures> {
        self.owner.disable()
    }
}

pub struct WidgetryPowerInhibitPlugin;

impl Plugin for WidgetryPowerInhibitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PowerInhibit>();
        widgetry_info!("WidgetryPowerInhibitPlugin 注册完成");
    }
}

trait Api {
    type Handle;

    fn create(&mut self) -> io::Result<Self::Handle>;
    fn set(&mut self, handle: &Self::Handle, kind: RequestKind) -> io::Result<()>;
    fn clear(&mut self, handle: &Self::Handle, kind: RequestKind) -> io::Result<()>;
}

struct Owner<A: Api> {
    api: A,
    request: Option<Request<A::Handle>>,
}

impl<A: Api + Default> Default for Owner<A> {
    fn default() -> Self {
        Self {
            api: A::default(),
            request: None,
        }
    }
}

struct Request<H> {
    handle: H,
    display: bool,
    system: bool,
}

impl<A: Api> Owner<A> {
    fn enable(&mut self) -> Result<(), PowerFailures> {
        if self
            .request
            .as_ref()
            .is_some_and(|request| request.display && request.system)
        {
            return Ok(());
        }
        self.disable()?;
        let mut request = Request {
            handle: self.api.create().map_err(|source| {
                PowerFailures::new(failure(
                    PowerOperation::Create,
                    PowerPhase::Enable,
                    None,
                    false,
                    false,
                    source,
                ))
            })?,
            display: false,
            system: false,
        };
        let result = self
            .api
            .set(&request.handle, RequestKind::Display)
            .map_err(|source| {
                failure(
                    PowerOperation::Set,
                    PowerPhase::Enable,
                    Some(RequestKind::Display),
                    request.display,
                    request.system,
                    source,
                )
            })
            .and_then(|()| {
                request.display = true;
                self.api
                    .set(&request.handle, RequestKind::System)
                    .map_err(|source| {
                        failure(
                            PowerOperation::Set,
                            PowerPhase::Enable,
                            Some(RequestKind::System),
                            request.display,
                            request.system,
                            source,
                        )
                    })
            });
        request.system = result.is_ok();
        self.request = Some(request);
        if let Err(error) = result {
            let mut errors = PowerFailures::new(error);
            if let Err(cleanup) = self.clear_requests(PowerPhase::Rollback) {
                errors.extend(cleanup);
            }
            // Closing the object also releases requests when rollback cannot clear them.
            self.request = None;
            return Err(errors);
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<(), PowerFailures> {
        self.clear_requests(PowerPhase::Disable)
    }

    fn clear_requests(&mut self, phase: PowerPhase) -> Result<(), PowerFailures> {
        let Some(request) = &mut self.request else {
            return Ok(());
        };
        let mut errors: Option<PowerFailures> = None;
        for kind in [RequestKind::Display, RequestKind::System] {
            let present = match kind {
                RequestKind::Display => request.display,
                RequestKind::System => request.system,
            };
            if present {
                match self.api.clear(&request.handle, kind) {
                    Ok(()) => match kind {
                        RequestKind::Display => request.display = false,
                        RequestKind::System => request.system = false,
                    },
                    Err(source) => {
                        let error = failure(
                            PowerOperation::Clear,
                            phase,
                            Some(kind),
                            request.display,
                            request.system,
                            source,
                        );
                        if let Some(errors) = &mut errors {
                            errors.push(error);
                        } else {
                            errors = Some(PowerFailures::new(error));
                        }
                    }
                }
            }
        }
        if let Some(error) = errors {
            return Err(error);
        }
        self.request = None;
        Ok(())
    }
}

impl<A: Api> Drop for Owner<A> {
    fn drop(&mut self) {
        if let Err(errors) = self.clear_requests(PowerPhase::Drop) {
            drop(self.request.take());
            for failure in errors.failures() {
                failure.record_handled(
                    "释放电源请求对象",
                    "清除调用失败，已交由句柄所有者执行最终关闭，未确认清除调用成功",
                );
            }
        }
    }
}

#[track_caller]
fn failure(
    operation: PowerOperation,
    phase: PowerPhase,
    request: Option<RequestKind>,
    display: bool,
    system: bool,
    source: io::Error,
) -> PowerFailure {
    let context = ErrorFrame::new("power request")
        .field("operation", format_args!("{operation:?}"))
        .field("phase", format_args!("{phase:?}"))
        .field("request", format_args!("{request:?}"))
        .field("display", display)
        .field("system", system);
    PowerFailure::new(
        PowerError {
            operation,
            phase,
            request,
            display,
            system,
            source,
        },
        context,
    )
}

#[allow(unsafe_code)]
mod native;

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests;
