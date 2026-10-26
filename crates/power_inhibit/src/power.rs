use bevy::prelude::*;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info, widgetry_warn};

pub type PowerError = BevyError;

#[derive(Resource, Default)]
pub struct PowerInhibit {
    owner: Owner<native::WindowsApi>,
}

impl PowerInhibit {
    pub fn enable(&mut self) -> Result<(), PowerError> {
        self.owner.enable().inspect_err(report_error)
    }

    pub fn disable(&mut self) -> Result<(), PowerError> {
        self.owner.disable().inspect_err(report_error)
    }
}

pub struct WidgetryPowerInhibitPlugin;

impl Plugin for WidgetryPowerInhibitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PowerInhibit>();
        widgetry_info!("WidgetryPowerInhibitPlugin 注册完成");
    }
}

fn report_error(error: &PowerError) {
    widgetry_error!(error = %error, "电源抑制操作失败");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RequestKind {
    Display,
    System,
}

trait Api {
    type Handle;

    fn create(&mut self) -> Result<Self::Handle, PowerError>;
    fn set(&mut self, handle: &Self::Handle, kind: RequestKind) -> Result<(), PowerError>;
    fn clear(&mut self, handle: &Self::Handle, kind: RequestKind) -> Result<(), PowerError>;
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
    fn enable(&mut self) -> Result<(), PowerError> {
        if self
            .request
            .as_ref()
            .is_some_and(|request| request.display && request.system)
        {
            return Ok(());
        }
        self.disable()?;
        let mut request = Request {
            handle: self.api.create()?,
            display: false,
            system: false,
        };
        let result = self
            .api
            .set(&request.handle, RequestKind::Display)
            .and_then(|()| {
                request.display = true;
                self.api.set(&request.handle, RequestKind::System)
            });
        request.system = result.is_ok();
        self.request = Some(request);
        if let Err(error) = result {
            let rollback = self.disable();
            // Closing the object also releases requests when rollback cannot clear them.
            self.request = None;
            return Err(match rollback {
                Ok(()) => error,
                Err(cleanup) => PowerError::error(format!("{error}; rollback: {cleanup}")),
            });
        }
        Ok(())
    }

    fn disable(&mut self) -> Result<(), PowerError> {
        let Some(request) = &mut self.request else {
            return Ok(());
        };
        let mut failure = None;
        for (present, kind) in [
            (&mut request.display, RequestKind::Display),
            (&mut request.system, RequestKind::System),
        ] {
            if *present {
                match self.api.clear(&request.handle, kind) {
                    Ok(()) => *present = false,
                    Err(error) => {
                        failure = Some(match failure {
                            None => error,
                            Some(previous) => PowerError::error(format!("{previous}; {error}")),
                        });
                    }
                }
            }
        }
        if let Some(error) = failure {
            return Err(error);
        }
        self.request = None;
        Ok(())
    }
}

impl<A: Api> Drop for Owner<A> {
    fn drop(&mut self) {
        if let Err(error) = self.disable() {
            widgetry_warn!(error = %error, "销毁电源抑制资源时清除请求失败，释放句柄回收剩余请求");
        }
    }
}

#[allow(unsafe_code)]
mod native;

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests;
