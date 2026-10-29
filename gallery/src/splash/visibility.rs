use bevy::window::RawHandleWrapper;
use raw_window_handle::RawWindowHandle;
use winsafe::{DwmAttr, HRGN, RECT, co, guard::DeleteObjectGuard};

pub(super) fn is_uncovered(
    handle: &RawHandleWrapper,
    monitors: impl IntoIterator<Item = RECT>,
) -> std::result::Result<bool, String> {
    let RawWindowHandle::Win32(raw) = handle.get_window_handle() else {
        return Err("主窗口缺少 Win32 handle".into());
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
    .map_err(|error| error.to_string())?;
    let Some(target) = target else {
        return Ok(false);
    };
    // The Gallery window is borderless, so DWM frame bounds are its client bounds in physical pixels.
    // GetClientRect/ClientToScreen may be DPI-virtualized while Monitor coordinates are physical.
    let DwmAttr::ExtendedFrameBounds(bounds) = target
        .DwmGetWindowAttribute(co::DWMWA::EXTENDED_FRAME_BOUNDS)
        .map_err(|error| error.to_string())?
    else {
        return Err("主窗口缺少 DWM 物理边界".into());
    };
    let remaining = on_screen_region(bounds, monitors).map_err(|error| error.to_string())?;
    if !remaining.RectInRegion(bounds) {
        return Ok(false);
    }
    let mut above = target
        .GetWindow(co::GW::HWNDFIRST)
        .map_err(|error| error.to_string())?;
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
                    .map_err(|error| error.to_string())?;
                if matches!(cloak, DwmAttr::Cloaked(flags) if flags.raw() == 0) {
                    let frame = above
                        .DwmGetWindowAttribute(co::DWMWA::EXTENDED_FRAME_BOUNDS)
                        .map_err(|error| error.to_string())?;
                    if let DwmAttr::ExtendedFrameBounds(rect) = frame {
                        let cover = HRGN::CreateRectRgn(rect).map_err(|error| error.to_string())?;
                        if remaining
                            .CombineRgn(&remaining, &cover, co::RGN::DIFF)
                            .map_err(|error| error.to_string())?
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
            .map_err(|error| error.to_string())?;
    }
    Err("检查期间窗口 Z-order 已改变".into())
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
