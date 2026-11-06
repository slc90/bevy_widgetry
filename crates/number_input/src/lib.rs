#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod editing;
mod error;
pub use error::{
    NumberInputConfigSnapshot, NumberInputError, NumberInputFailure, NumberInputRejection,
    NumberInputValue,
};
mod headless;
mod input;
pub use headless::{WidgetryNumberInputEditor, WidgetryNumberInputHeadless};
pub use input::WidgetryNumberInputStep;
mod layout;
mod number;
mod repeating;
pub use repeating::WidgetryNumberInputRepeat;
mod style;
mod value;

pub use number::WidgetryNumber;
pub use value::{WidgetryNumberInput, WidgetryNumberInputProps};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_debug;

pub struct WidgetryNumberInputPlugin;

pub struct WidgetryNumberInputHeadlessPlugin;

impl Plugin for WidgetryNumberInputHeadlessPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<bevy_widgetry_text_field::WidgetryTextFieldHeadlessPlugin>() {
            app.add_plugins(bevy_widgetry_text_field::WidgetryTextFieldHeadlessPlugin);
        }
        if !app.is_plugin_added::<bevy::ui_widgets::ButtonPlugin>() {
            app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        }
        input::install_focus(app);
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
        widgetry_debug!("WidgetryNumberInputHeadlessPlugin 注册完成");
    }
}

impl Plugin for WidgetryNumberInputPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryNumberInputHeadlessPlugin>() {
            app.add_plugins(WidgetryNumberInputHeadlessPlugin);
        }
        if !app.is_plugin_added::<bevy_widgetry_text_field::WidgetryTextFieldPlugin>() {
            app.add_plugins(bevy_widgetry_text_field::WidgetryTextFieldPlugin);
        }
        layout::install(app);
        style::install(app);
        widgetry_debug!("WidgetryNumberInputPlugin 注册完成");
    }
}
