use bevy::window::RawHandleWrapper;
use raw_window_handle::RawWindowHandle;
use winsafe::{DwmAttr, HRGN, RECT, co, guard::DeleteObjectGuard};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum VisibilityError {
    Native(winsafe::co::ERROR),
    Dwm(winsafe::co::HRESULT),
    MissingHandle,
    MissingBounds,
    ZOrderChanged,
}
impl std::fmt::Display for VisibilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(_) | Self::Dwm(_) => f.write_str("Splash native visibility query failed"),
            Self::MissingHandle => f.write_str("Splash window has no Win32 handle"),
            Self::MissingBounds => f.write_str("Splash window has no DWM physical bounds"),
            Self::ZOrderChanged => {
                f.write_str("Window Z-order changed during Splash visibility query")
            }
        }
    }
}
impl std::error::Error for VisibilityError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Native(source) => Some(source),
            Self::Dwm(source) => Some(source),
            _ => None,
        }
    }
}

impl From<winsafe::co::ERROR> for VisibilityError {
    fn from(error: winsafe::co::ERROR) -> Self {
        Self::Native(error)
    }
}
impl From<winsafe::co::HRESULT> for VisibilityError {
    fn from(error: winsafe::co::HRESULT) -> Self {
        Self::Dwm(error)
    }
}

pub(super) fn is_uncovered(
    handle: &RawHandleWrapper,
    monitors: impl IntoIterator<Item = RECT>,
) -> std::result::Result<bool, VisibilityError> {
    let RawWindowHandle::Win32(raw) = handle.get_window_handle() else {
        return Err(VisibilityError::MissingHandle);
    };
    let mut target = None;
    let mut window_count = 0;
    // EnumWindows supplies borrowed native handles without constructing one from an unchecked raw pointer.
    winsafe::EnumWindows(|window| {
        window_count += 1;
        if window.ptr() as isize == raw.hwnd.get() {
            target = Some(window);
        }
        true
    })
    .map_err(VisibilityError::from)?;
    let Some(target) = target else {
        return Ok(false);
    };
    // The Gallery window is borderless, so DWM frame bounds are its client bounds in physical pixels.
    // GetClientRect/ClientToScreen may be DPI-virtualized while Monitor coordinates are physical.
    let DwmAttr::ExtendedFrameBounds(bounds) = target
        .DwmGetWindowAttribute(co::DWMWA::EXTENDED_FRAME_BOUNDS)
        .map_err(VisibilityError::from)?
    else {
        return Err(VisibilityError::MissingBounds);
    };
    let remaining = on_screen_region(bounds, monitors).map_err(VisibilityError::from)?;
    if !remaining.RectInRegion(bounds) {
        return Ok(false);
    }
    let mut above = target
        .GetWindow(co::GW::HWNDFIRST)
        .map_err(VisibilityError::from)?;
    for _ in 0..window_count {
        if above == target {
            return Ok(true);
        }
        if above.IsWindowVisible() && !above.IsIconic() {
            let styles = above.GetWindowLongPtr(co::GWLP::EXSTYLE);
            // Per-pixel layered windows cannot be treated as opaque rectangles.
            if styles & (co::WS_EX::LAYERED.raw() | co::WS_EX::TRANSPARENT.raw()) as isize == 0 {
                let cloak = above
                    .DwmGetWindowAttribute(co::DWMWA::CLOAKED)
                    .map_err(VisibilityError::from)?;
                if matches!(cloak, DwmAttr::Cloaked(flags) if flags.raw() == 0) {
                    let frame = above
                        .DwmGetWindowAttribute(co::DWMWA::EXTENDED_FRAME_BOUNDS)
                        .map_err(VisibilityError::from)?;
                    if let DwmAttr::ExtendedFrameBounds(rect) = frame {
                        let cover = HRGN::CreateRectRgn(rect).map_err(VisibilityError::from)?;
                        if remaining
                            .CombineRgn(&remaining, &cover, co::RGN::DIFF)
                            .map_err(VisibilityError::from)?
                            == co::REGION::NULL
                        {
                            return Ok(false);
                        }
                    }
                }
            }
        }
        above = above
            .GetWindow(co::GW::HWNDNEXT)
            .map_err(VisibilityError::from)?;
    }
    Err(VisibilityError::ZOrderChanged)
}

fn on_screen_region(
    bounds: RECT,
    monitors: impl IntoIterator<Item = RECT>,
) -> winsafe::SysResult<DeleteObjectGuard<HRGN>> {
    let remaining = HRGN::CreateRectRgn(bounds)?;
    let screens = HRGN::CreateRectRgn(RECT::new())?;
    for monitor in monitors {
        let region = HRGN::CreateRectRgn(monitor)?;
        screens.CombineRgn(&screens, &region, co::RGN::OR)?;
    }
    remaining.CombineRgn(&remaining, &screens, co::RGN::AND)?;
    Ok(remaining)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    fn rect(left: i32, top: i32, right: i32, bottom: i32) -> RECT {
        RECT {
            left,
            top,
            right,
            bottom,
        }
    }

    #[test]
    fn covered_visible_part_does_not_leave_off_screen_pixels() {
        let client = rect(-40, -20, 140, 120);
        let monitor = rect(0, 0, 100, 100);
        let remaining = on_screen_region(client, [monitor]).unwrap();
        let cover = HRGN::CreateRectRgn(monitor).unwrap();
        assert_eq!(
            remaining
                .CombineRgn(&remaining, &cover, co::RGN::DIFF)
                .unwrap(),
            co::REGION::NULL
        );
    }

    #[test]
    fn gaps_between_monitors_are_not_visible() {
        let client = rect(80, 0, 220, 100);
        let monitors = [rect(0, 0, 100, 100), rect(200, 0, 300, 100)];
        let remaining = on_screen_region(client, monitors).unwrap();
        for monitor in monitors {
            let cover = HRGN::CreateRectRgn(monitor).unwrap();
            remaining
                .CombineRgn(&remaining, &cover, co::RGN::DIFF)
                .unwrap();
        }
        assert!(!remaining.RectInRegion(client));
    }

    #[test]
    fn partial_cover_preserves_visible_pixels_on_another_monitor() {
        let client = rect(80, 0, 220, 100);
        let monitors = [rect(0, 0, 100, 100), rect(200, 0, 300, 100)];
        let remaining = on_screen_region(client, monitors).unwrap();
        let cover = HRGN::CreateRectRgn(monitors[0]).unwrap();
        remaining
            .CombineRgn(&remaining, &cover, co::RGN::DIFF)
            .unwrap();
        assert!(remaining.PtInRegion(210, 50));
        assert!(!remaining.PtInRegion(90, 50));
    }

    #[test]
    fn entirely_off_screen_window_has_no_visible_region() {
        let client = rect(110, 0, 180, 100);
        let remaining = on_screen_region(client, [rect(0, 0, 100, 100)]).unwrap();
        assert!(!remaining.RectInRegion(client));
    }
}
