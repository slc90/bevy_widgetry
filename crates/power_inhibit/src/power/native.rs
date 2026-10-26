use super::{Api, PowerError, RequestKind};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
use windows_sys::Win32::System::Power::{
    PowerClearRequest, PowerCreateRequest, PowerRequestDisplayRequired, PowerRequestSystemRequired,
    PowerSetRequest,
};
use windows_sys::Win32::System::Threading::{
    POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0,
};

#[derive(Default)]
pub(super) struct WindowsApi;

impl Api for WindowsApi {
    // OwnedHandle is Send + Sync and calls CloseHandle exactly once on drop.
    // Power request handles are process-owned kernel objects, with no thread affinity.
    // The owner keeps the handle private and mutates request counts only through &mut self.
    type Handle = OwnedHandle;

    fn create(&mut self) -> Result<Self::Handle, PowerError> {
        let mut reason: Vec<u16> = "Widgetry 应用正在执行需要保持显示器和系统运行的任务"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let context = REASON_CONTEXT {
            Version: 0,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: reason.as_mut_ptr(),
            },
        };
        // SAFETY: Context uses version 0 and the simple-string union variant.
        // Its null-terminated UTF-16 buffer and the context remain alive for the call.
        let handle = unsafe { PowerCreateRequest(&context) };
        if handle == INVALID_HANDLE_VALUE {
            return Err(last_error("PowerCreateRequest", None));
        }
        // SAFETY: The API returned a valid, exclusively owned kernel handle.
        // Ownership is transferred exactly once and no raw handle escapes this module.
        Ok(unsafe { OwnedHandle::from_raw_handle(handle) })
    }

    fn set(&mut self, handle: &Self::Handle, kind: RequestKind) -> Result<(), PowerError> {
        let request_type = request_type(kind);
        // SAFETY: The borrowed owner keeps the power request handle valid for the call.
        // The request type is one of the two supported power request types.
        if unsafe { PowerSetRequest(handle.as_raw_handle(), request_type) } == 0 {
            return Err(last_error("PowerSetRequest", Some(kind)));
        }
        Ok(())
    }

    fn clear(&mut self, handle: &Self::Handle, kind: RequestKind) -> Result<(), PowerError> {
        let request_type = request_type(kind);
        // SAFETY: The owner retains the handle and only asks to clear successfully set types.
        // Exclusive owner access prevents simultaneous count changes or handle release.
        if unsafe { PowerClearRequest(handle.as_raw_handle(), request_type) } == 0 {
            return Err(last_error("PowerClearRequest", Some(kind)));
        }
        Ok(())
    }
}

fn request_type(kind: RequestKind) -> i32 {
    match kind {
        RequestKind::Display => PowerRequestDisplayRequired,
        RequestKind::System => PowerRequestSystemRequired,
    }
}

fn last_error(operation: &str, kind: Option<RequestKind>) -> PowerError {
    let error = std::io::Error::last_os_error();
    PowerError::error(format!("{operation} ({kind:?}): {error}"))
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use crate::{PowerInhibit, WidgetryPowerInhibitPlugin};
    use bevy::prelude::*;
    use windows_sys::Win32::Foundation::{
        ERROR_INVALID_HANDLE, GetHandleInformation, GetLastError,
    };

    // Serialize native handle use so another test cannot reuse a just-closed handle value.
    static NATIVE_REQUESTS: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn native_requests_close_on_disable_resource_removal_and_app_drop() {
        let _guard = NATIVE_REQUESTS.lock().unwrap();
        for cleanup in 0..3 {
            let mut app = App::new();
            app.add_plugins(WidgetryPowerInhibitPlugin);
            app.world_mut()
                .resource_mut::<PowerInhibit>()
                .enable()
                .unwrap();
            let handle = app
                .world()
                .resource::<PowerInhibit>()
                .owner
                .request
                .as_ref()
                .unwrap()
                .handle
                .as_raw_handle();
            let mut flags = 0;
            // SAFETY: The resource owns a live power request handle and flags is writable.
            assert_ne!(unsafe { GetHandleInformation(handle, &mut flags) }, 0);
            match cleanup {
                0 => app
                    .world_mut()
                    .resource_mut::<PowerInhibit>()
                    .disable()
                    .unwrap(),
                1 => {
                    app.world_mut().remove_resource::<PowerInhibit>();
                }
                _ => drop(app),
            }
            // SAFETY: GetHandleInformation reports invalid handles without dereferencing them.
            assert_eq!(unsafe { GetHandleInformation(handle, &mut flags) }, 0);
            // SAFETY: No intervening Win32 call has overwritten the thread's error code.
            assert_eq!(unsafe { GetLastError() }, ERROR_INVALID_HANDLE);
        }
    }

    #[test]
    fn native_request_can_be_enabled_and_disabled_on_different_threads() {
        let _guard = NATIVE_REQUESTS.lock().unwrap();
        let mut power = std::thread::spawn(|| {
            let mut power = PowerInhibit::default();
            power.enable().unwrap();
            power
        })
        .join()
        .unwrap();
        power.enable().unwrap();
        power.disable().unwrap();
    }

    #[test]
    fn native_error_keeps_api_type_win32_code_and_error_severity() {
        let _guard = NATIVE_REQUESTS.lock().unwrap();
        let mut api = WindowsApi;
        let handle = api.create().unwrap();
        let error = api.clear(&handle, RequestKind::Display).unwrap_err();
        assert_eq!(error.severity(), bevy::ecs::error::Severity::Error);
        assert!(error.to_string().contains("PowerClearRequest"));
        assert!(error.to_string().contains("Display"));
        assert!(error.to_string().contains("os error"));
    }
}
