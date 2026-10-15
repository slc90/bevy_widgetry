#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

pub mod benchmark;
mod error;
mod logging;
mod pointer;
mod scene;
mod theme;

pub use error::ErrorCapture;
pub use logging::{LogCapture, LogRecord};
pub use pointer::{
    cancel, drag_end, pointer_event, pointer_ids, press, primary_cancel, primary_click,
    primary_drag_end, primary_press, primary_release, queue_pointer, release,
};
pub use scene::{
    add_keyboard_dispatch, add_ui_plugins, advance_until, picking_app, press_key, queue_key,
    scene_app, spawn_picking_camera, spawn_ui_camera, text_edit_app, text_input_app,
};
pub use theme::switch_theme;
