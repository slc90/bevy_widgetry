#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod editing;
mod input;
mod layout;
mod number;
mod repeating;
mod value;

pub use number::WidgetryNumber;
pub use value::{WidgetryNumberInput, WidgetryNumberInputProps};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryNumberInputPlugin;

impl Plugin for WidgetryNumberInputPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_text_field::WidgetryTextFieldPlugin>() {
            app.add_plugins(bevy_widgetry_text_field::WidgetryTextFieldPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        input::install_focus(app);
        layout::install(app);
        repeating::install(app);
        macro_rules! register {
            ($($t:ty),+ $(,)?) => {$(
                app.register_type::<value::State<$t>>();
                editing::install::<$t>(app);
                input::install::<$t>(app);
            )+};
        }
        register!(
            i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
        );
        widgetry_info!("WidgetryNumberInputPlugin 注册完成");
    }
}
