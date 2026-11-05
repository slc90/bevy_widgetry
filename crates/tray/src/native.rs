use crate::bridge::Bridge;
use crate::{TrayError, TrayFailure};
use bevy_widgetry_diagnostics::ErrorFrame;
use std::cell::RefCell;
use std::io;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tray_icon::TrayIcon;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, EndMenu, HHOOK, MSG, MSGF_MENU, SetWindowsHookExW, UnhookWindowsHookEx,
    WH_MSGFILTER, WM_ENTERMENULOOP, WM_EXITMENULOOP, WM_LBUTTONDOWN, WM_LBUTTONUP,
    WM_NCLBUTTONDOWN, WM_NCLBUTTONUP,
};

#[derive(Default)]
pub(crate) struct MenuGesture {
    open: AtomicBool,
    closing: AtomicBool,
    enabled: AtomicBool,
    pressed: AtomicBool,
}

#[derive(Debug, PartialEq, Eq)]
enum MenuInput {
    Pass,
    Consume,
    Close,
}

impl MenuGesture {
    pub(crate) fn tray_input(&self, state: tray_icon::MouseButtonState) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return state == tray_icon::MouseButtonState::Up;
        }
        match state {
            tray_icon::MouseButtonState::Down => {
                self.pressed.store(self.allows_click(), Ordering::Relaxed);
                false
            }
            tray_icon::MouseButtonState::Up => {
                self.pressed.swap(false, Ordering::Relaxed) && self.allows_click()
            }
        }
    }

    pub(crate) fn allows_click(&self) -> bool {
        !self.open.load(Ordering::Relaxed)
    }

    fn filter(&self, message: u32, point: POINT, rect: Option<tray_icon::Rect>) -> MenuInput {
        if !self.open.load(Ordering::Relaxed) {
            return MenuInput::Pass;
        }
        if self.closing.load(Ordering::Relaxed) {
            return match message {
                WM_LBUTTONUP | WM_NCLBUTTONUP => MenuInput::Close,
                WM_LBUTTONDOWN | WM_NCLBUTTONDOWN => MenuInput::Consume,
                _ => MenuInput::Pass,
            };
        }
        if matches!(message, WM_LBUTTONDOWN | WM_NCLBUTTONDOWN)
            && rect.is_some_and(|rect| {
                let x = f64::from(point.x);
                let y = f64::from(point.y);
                x >= rect.position.x
                    && x < rect.position.x + f64::from(rect.size.width)
                    && y >= rect.position.y
                    && y < rect.position.y + f64::from(rect.size.height)
            })
        {
            self.closing.store(true, Ordering::Relaxed);
            return MenuInput::Consume;
        }
        MenuInput::Pass
    }
}

struct Context {
    icon: TrayIcon,
    bridge: Arc<Bridge>,
}
thread_local! { static SESSIONS: RefCell<Vec<Weak<Context>>> = const { RefCell::new(Vec::new()) }; }
pub(crate) struct MenuSession {
    context: Rc<Context>,
    hook: HHOOK,
}

impl MenuSession {
    pub(crate) fn new(icon: &TrayIcon, bridge: &Arc<Bridge>) -> Result<Self, TrayFailure> {
        let pointer = Arc::as_ptr(bridge) as usize;
        // SAFETY: The hook handles only native modal menu input on this creating thread.
        let hook = unsafe {
            SetWindowsHookExW(
                WH_MSGFILTER,
                Some(menu_filter),
                std::ptr::null_mut(),
                GetCurrentThreadId(),
            )
        };
        if hook.is_null() {
            return Err(failure(
                "install menu input filter",
                io::Error::last_os_error(),
            ));
        }
        // SAFETY: NonSend ownership keeps callbacks, HWND ownership and removal on the same thread.
        if unsafe { SetWindowSubclass(icon.window_handle(), Some(menu_proc), pointer, pointer) }
            == 0
        {
            let error = failure("install menu subclass", io::Error::last_os_error());
            // SAFETY: This is the successfully installed hook owned by this call.
            if unsafe { UnhookWindowsHookEx(hook) } == 0 {
                bridge.fail(failure(
                    "remove failed menu initialization filter",
                    io::Error::last_os_error(),
                ));
            }
            return Err(error);
        }
        let context = Rc::new(Context {
            icon: icon.clone(),
            bridge: bridge.clone(),
        });
        bridge.gesture.enabled.store(true, Ordering::Relaxed);
        SESSIONS.with(|sessions| sessions.borrow_mut().push(Rc::downgrade(&context)));
        Ok(Self { context, hook })
    }
}

impl Drop for MenuSession {
    fn drop(&mut self) {
        SESSIONS.with(|sessions| {
            sessions
                .borrow_mut()
                .retain(|session| !std::ptr::eq(session.as_ptr(), Rc::as_ptr(&self.context)))
        });
        let bridge = &self.context.bridge;
        // SAFETY: This owner installed the hook and removes it once on the same thread.
        if unsafe { UnhookWindowsHookEx(self.hook) } == 0 {
            bridge.fail(failure(
                "remove menu input filter",
                io::Error::last_os_error(),
            ));
        }
        let pointer = Arc::as_ptr(bridge) as usize;
        // SAFETY: Context still owns the live HWND and callback data.
        if unsafe {
            RemoveWindowSubclass(self.context.icon.window_handle(), Some(menu_proc), pointer)
        } == 0
        {
            bridge.fail(failure("remove menu subclass", io::Error::last_os_error()));
            // Retain callback data if Windows cannot detach it before HWND destruction.
            std::mem::forget(bridge.clone());
        }
        bridge.gesture.open.store(false, Ordering::Relaxed);
        bridge.gesture.closing.store(false, Ordering::Relaxed);
        bridge.gesture.enabled.store(false, Ordering::Relaxed);
        bridge.gesture.pressed.store(false, Ordering::Relaxed);
    }
}

unsafe extern "system" fn menu_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    // SAFETY: MenuSession retains the bridge until after removing this subclass.
    let bridge = unsafe { &*(data as *const Bridge) };
    match message {
        WM_ENTERMENULOOP => {
            bridge.gesture.pressed.store(false, Ordering::Relaxed);
            bridge.gesture.closing.store(false, Ordering::Relaxed);
            bridge.gesture.open.store(true, Ordering::Relaxed);
        }
        WM_EXITMENULOOP => {
            bridge.gesture.open.store(false, Ordering::Relaxed);
            bridge.gesture.closing.store(false, Ordering::Relaxed);
        }
        _ => {}
    }
    // SAFETY: Forward the unchanged message through the existing subclass chain.
    unsafe { DefSubclassProc(hwnd, message, wparam, lparam) }
}

unsafe extern "system" fn menu_filter(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == MSGF_MENU as i32 {
        // SAFETY: MSGF_MENU supplies a live MSG during this callback.
        let message = unsafe { &*(lparam as *const MSG) };
        // Release the RefCell borrow before native calls may reenter the hook.
        let contexts: Vec<_> =
            SESSIONS.with(|sessions| sessions.borrow().iter().filter_map(Weak::upgrade).collect());
        for context in contexts {
            if context.bridge.gesture.allows_click() {
                continue;
            }
            let rect = if matches!(message.message, WM_LBUTTONDOWN | WM_NCLBUTTONDOWN) {
                context.icon.rect()
            } else {
                None
            };
            match context
                .bridge
                .gesture
                .filter(message.message, message.pt, rect)
            {
                MenuInput::Pass => {}
                MenuInput::Consume => return 1,
                MenuInput::Close => {
                    // SAFETY: This is the owning thread's active native menu input loop.
                    // Both halves of the closing gesture are consumed before reaching Explorer.
                    if unsafe { EndMenu() } == 0 {
                        context
                            .bridge
                            .fail(failure("end tray menu gesture", io::Error::last_os_error()));
                    }
                    return 1;
                }
            }
        }
    }
    // SAFETY: Unrelated input and negative hook codes pass through unchanged.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

fn failure(operation: &'static str, source: io::Error) -> TrayFailure {
    TrayFailure::new(
        TrayError::Native { operation, source },
        ErrorFrame::new(operation),
    )
}

pub(crate) fn verify_menu(
    menu: &tray_icon::menu::Menu,
    entries: &[crate::TrayMenuEntry],
) -> Result<(), TrayFailure> {
    use tray_icon::menu::ContextMenu;
    // SAFETY: The borrowed menu owns this root and all its submenu handles on the current thread.
    unsafe { verify_menu_handle(menu.hpopupmenu() as _, entries) }
}

unsafe fn verify_menu_handle(
    handle: windows_sys::Win32::UI::WindowsAndMessaging::HMENU,
    entries: &[crate::TrayMenuEntry],
) -> Result<(), TrayFailure> {
    use crate::TrayMenuEntry;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetMenuItemCount, GetMenuItemInfoW, MENUITEMINFOW, MFT_SEPARATOR, MIIM_FTYPE, MIIM_STRING,
        MIIM_SUBMENU,
    };
    // SAFETY: The caller retains the owning root throughout recursive verification.
    let count = unsafe { GetMenuItemCount(handle) };
    if count < 0 {
        return Err(failure(
            "verify native tray menu",
            io::Error::last_os_error(),
        ));
    }
    for (index, entry) in entries.iter().enumerate() {
        let mut info = MENUITEMINFOW {
            cbSize: std::mem::size_of::<MENUITEMINFOW>() as _,
            fMask: MIIM_SUBMENU | MIIM_STRING | MIIM_FTYPE,
            ..Default::default()
        };
        // SAFETY: Info is writable and initialized with its required size; positions are checked by Windows.
        if unsafe { GetMenuItemInfoW(handle, index as u32, 1, &mut info) } == 0 {
            return Err(failure(
                "verify inserted tray menu item",
                io::Error::last_os_error(),
            ));
        }
        let (expected_text, children) = match entry {
            TrayMenuEntry::Item { text, .. } => (Some(text.as_str()), None),
            TrayMenuEntry::Separator => (None, None),
            TrayMenuEntry::Submenu { text, entries } => (Some(text.as_str()), Some(entries)),
        };
        if (info.fType & MFT_SEPARATOR != 0) != expected_text.is_none()
            || info.hSubMenu.is_null() == children.is_some()
        {
            return Err(projection_failure(
                index,
                "native item kind differs from requested tree",
            ));
        }
        if let Some(expected) = expected_text {
            let mut text = vec![0u16; info.cch as usize + 1];
            info.dwTypeData = text.as_mut_ptr();
            info.cch = text.len() as u32;
            // SAFETY: The live buffer is large enough for the size just queried, including the terminator.
            if unsafe { GetMenuItemInfoW(handle, index as u32, 1, &mut info) } == 0 {
                return Err(failure(
                    "read native tray menu label",
                    io::Error::last_os_error(),
                ));
            }
            if String::from_utf16_lossy(&text[..info.cch as usize]) != expected {
                return Err(projection_failure(
                    index,
                    "native label differs from requested text",
                ));
            }
        }
        if let Some(children) = children {
            // SAFETY: The root owns the submenu handle returned by GetMenuItemInfoW.
            unsafe { verify_menu_handle(info.hSubMenu, children) }?;
        }
    }
    if count as usize != entries.len() {
        return Err(projection_failure(
            entries.len(),
            "native item count differs from requested tree",
        ));
    }
    Ok(())
}

fn projection_failure(index: usize, detail: &'static str) -> TrayFailure {
    TrayFailure::new(
        TrayError::MenuProjection { detail },
        ErrorFrame::new("verify native tray menu projection").field("index", index),
    )
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

    fn rect() -> tray_icon::Rect {
        tray_icon::Rect {
            position: tray_icon::dpi::PhysicalPosition::new(10.0, 20.0),
            size: tray_icon::dpi::PhysicalSize::new(16, 16),
        }
    }

    #[test]
    fn closing_gesture_is_consumed_inside_menu_before_shell_notification() {
        let gesture = MenuGesture::default();
        gesture.open.store(true, Ordering::Relaxed);
        let point = POINT { x: 12, y: 22 };
        assert_eq!(
            gesture.filter(WM_LBUTTONDOWN, point, Some(rect())),
            MenuInput::Consume
        );
        assert!(!gesture.allows_click());
        assert_eq!(gesture.filter(WM_LBUTTONUP, point, None), MenuInput::Close);
        gesture.open.store(false, Ordering::Relaxed);
        assert!(gesture.allows_click());
        assert_eq!(
            gesture.filter(WM_LBUTTONDOWN, point, Some(rect())),
            MenuInput::Pass
        );
        assert_eq!(gesture.filter(WM_LBUTTONUP, point, None), MenuInput::Pass);
    }

    #[test]
    fn other_positions_and_escape_keep_normal_native_input() {
        let gesture = MenuGesture::default();
        gesture.open.store(true, Ordering::Relaxed);
        assert_eq!(
            gesture.filter(WM_LBUTTONDOWN, POINT { x: 100, y: 200 }, Some(rect())),
            MenuInput::Pass
        );
        assert_eq!(
            gesture.filter(WM_LBUTTONUP, POINT { x: 100, y: 200 }, None),
            MenuInput::Pass
        );
        assert_eq!(
            gesture.filter(
                windows_sys::Win32::UI::WindowsAndMessaging::WM_KEYDOWN,
                POINT::default(),
                None
            ),
            MenuInput::Pass
        );
        gesture.open.store(false, Ordering::Relaxed);
        assert!(gesture.allows_click());
    }

    #[test]
    fn early_menu_exit_cannot_turn_the_consumed_press_into_a_restore_release() {
        use tray_icon::MouseButtonState::{Down, Up};
        let gesture = MenuGesture::default();
        gesture.enabled.store(true, Ordering::Relaxed);
        gesture.open.store(true, Ordering::Relaxed);
        let point = POINT { x: 12, y: 22 };
        assert_eq!(
            gesture.filter(WM_LBUTTONDOWN, point, Some(rect())),
            MenuInput::Consume
        );
        // Escape or native focus loss can exit the menu before the physical release.
        gesture.open.store(false, Ordering::Relaxed);
        gesture.closing.store(false, Ordering::Relaxed);
        assert!(!gesture.tray_input(Up));
        assert!(!gesture.tray_input(Down));
        assert!(gesture.tray_input(Up));
        assert!(!gesture.tray_input(Up));
    }

    #[test]
    fn menu_open_press_never_arms_a_later_release_and_no_menu_keeps_legacy_behavior() {
        use tray_icon::MouseButtonState::{Down, Up};
        let gesture = MenuGesture::default();
        assert!(gesture.tray_input(Up));
        gesture.enabled.store(true, Ordering::Relaxed);
        gesture.open.store(true, Ordering::Relaxed);
        assert!(!gesture.tray_input(Down));
        gesture.open.store(false, Ordering::Relaxed);
        assert!(!gesture.tray_input(Up));
        assert!(!gesture.tray_input(Down));
        assert!(gesture.tray_input(Up));
    }

    #[test]
    fn native_projection_verification_detects_items_missing_from_os_menu() {
        use crate::{TrayMenuEntry, WidgetryTrayMenu};
        use tray_icon::menu::ContextMenu;
        use windows_sys::Win32::UI::WindowsAndMessaging::{MF_BYPOSITION, RemoveMenu};
        let model = WidgetryTrayMenu::new(vec![TrayMenuEntry::item("open", "Open")]).unwrap();
        let menu = crate::menu::NativeMenu::new(&model).unwrap();
        // SAFETY: The test owns the live menu and removes only its native projection.
        assert_ne!(
            unsafe { RemoveMenu(menu.root.hpopupmenu() as _, 0, MF_BYPOSITION) },
            0
        );
        let error = verify_menu(&menu.root, model.entries()).unwrap_err();
        assert!(
            matches!(error.kind(), TrayError::Native { source, .. } if source.raw_os_error().is_some())
        );
        assert_eq!(menu.root.items().len(), 1);
    }

    #[test]
    fn native_lifecycle_registers_and_releases_filter_and_subclass() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{CallMsgFilterW, SendMessageW};
        let icon = tray_icon::TrayIconBuilder::new()
            .with_icon(tray_icon::Icon::from_rgba(vec![255; 16 * 16 * 4], 16, 16).unwrap())
            .with_menu(Box::new(tray_icon::menu::Menu::new()))
            .with_menu_on_left_click(false)
            .build()
            .unwrap();
        let (bridge, inbox) = Bridge::new(icon.id().clone(), || Ok(()));
        let session = MenuSession::new(&icon, &bridge).unwrap();
        // SAFETY: The test owns this window and sends pointer-free lifecycle notifications.
        unsafe {
            SendMessageW(icon.window_handle(), WM_ENTERMENULOOP, 1, 0);
        }
        assert!(!bridge.gesture.allows_click());
        if let Some(rect) = icon.rect() {
            let message = MSG {
                message: WM_LBUTTONDOWN,
                pt: POINT {
                    x: rect.position.x as i32,
                    y: rect.position.y as i32,
                },
                ..Default::default()
            };
            // SAFETY: This MSG lives through synchronous dispatch and does not request EndMenu.
            assert_ne!(unsafe { CallMsgFilterW(&message, MSGF_MENU as i32) }, 0);
        }
        // SAFETY: The window is alive throughout both calls, including after subclass removal.
        unsafe {
            SendMessageW(icon.window_handle(), WM_EXITMENULOOP, 1, 0);
            assert!(bridge.gesture.allows_click());
            drop(session);
            SendMessageW(icon.window_handle(), WM_ENTERMENULOOP, 1, 0);
            assert!(bridge.gesture.allows_click());
        }
        assert!(crate::bridge::forward_errors(&inbox).is_ok());
    }
}
