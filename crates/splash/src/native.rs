use crate::{SplashError, SplashFailure, error::failure};
use bevy::{prelude::*, window::RawHandleWrapper};
use raw_window_handle::RawWindowHandle;
use winsafe::{DwmAttr, HWND};

fn window_handle(world: &World, window: Entity) -> Result<HWND, SplashFailure> {
    let missing = || {
        failure(
            SplashError::NativeHandleMissing { window },
            "splash.handoff.handle",
        )
    };
    let raw = world.get::<RawHandleWrapper>(window).ok_or_else(missing)?;
    let RawWindowHandle::Win32(raw) = raw.get_window_handle() else {
        return Err(missing());
    };
    let mut found = None;
    // EnumWindows provides a borrowed HWND without unchecked raw handle construction.
    winsafe::EnumWindows(|handle| {
        if handle.ptr() as isize == raw.hwnd.get() {
            found = Some(handle);
        }
        true
    })
    .map_err(|source| {
        failure(
            SplashError::NativeWindowQuery { window, source },
            "splash.handoff.handle",
        )
    })?;
    found.ok_or_else(missing)
}

pub(crate) fn show_main(world: &mut World, window: Entity) -> Result<(), Vec<SplashFailure>> {
    let handle = window_handle(world, window).map_err(|error| vec![error])?;
    set_transitions(&handle, window, true).map_err(|error| vec![error])?;
    // DWM's transition flag is write-only. Resume normal transitions after startup Show.
    finish_show(
        crate::presentation::show_window(world, window, true),
        || {
            winsafe::DwmFlush().map_err(|source| {
                failure(
                    SplashError::Presentation { window, source },
                    "splash.handoff.flush",
                )
            })
        },
        || set_transitions(&handle, window, false),
    )
}

fn finish_show(
    show: Result<(), SplashFailure>,
    flush: impl FnOnce() -> Result<(), SplashFailure>,
    restore: impl FnOnce() -> Result<(), SplashFailure>,
) -> Result<(), Vec<SplashFailure>> {
    let mut failures = Vec::new();
    match show {
        Ok(()) => {
            if let Err(error) = flush() {
                failures.push(error);
            }
        }
        Err(error) => failures.push(error),
    }
    if let Err(error) = restore() {
        failures.push(error);
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures)
    }
}

fn set_transitions(handle: &HWND, window: Entity, disabled: bool) -> Result<(), SplashFailure> {
    handle
        .DwmSetWindowAttribute(DwmAttr::TransitionsForceDisabled(disabled))
        .map_err(|source| {
            failure(
                SplashError::WindowTransition {
                    window,
                    disabled,
                    source,
                },
                "splash.handoff.transitions",
            )
        })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::{cell::Cell, error::Error};

    #[test]
    fn show_failure_skips_flush_but_still_resumes_normal_transitions() {
        let restored = Cell::new(false);
        let error = failure(
            SplashError::WindowMissing {
                window: Entity::PLACEHOLDER,
            },
            "splash.test.show",
        );
        let id = error.origin().id().clone();
        let errors = finish_show(
            Err(error),
            || panic!("failed Show must not flush"),
            || {
                restored.set(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert!(restored.get());
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].origin().id(), &id);
    }

    #[test]
    fn flush_and_restoration_failures_are_both_preserved() {
        let flush = failure(
            SplashError::Presentation {
                window: Entity::PLACEHOLDER,
                source: winsafe::co::HRESULT::E_FAIL,
            },
            "splash.test.flush",
        );
        let restore = failure(
            SplashError::WindowTransition {
                window: Entity::PLACEHOLDER,
                disabled: false,
                source: winsafe::co::HRESULT::E_INVALIDARG,
            },
            "splash.test.restore",
        );
        let ids = [flush.origin().id().clone(), restore.origin().id().clone()];
        let errors = finish_show(Ok(()), || Err(flush), || Err(restore)).unwrap_err();
        assert_eq!(errors.len(), 2);
        for (error, id) in errors.iter().zip(ids) {
            assert_eq!(error.origin().id(), &id);
            assert!(error.kind().source().unwrap().is::<winsafe::co::HRESULT>());
        }
    }
}
