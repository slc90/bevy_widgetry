#[cfg(not(all(target_os = "windows", target_pointer_width = "64")))]
compile_error!("bevy_widgetry 仅支持 Windows 64 位 target");

mod number;
mod value;

pub use number::WidgetryNumber;
pub use value::{WidgetryNumberInput, WidgetryNumberInputProps};

use bevy::prelude::*;
use bevy_widgetry_internal_log::widgetry_info;

pub struct WidgetryNumberInputPlugin;

impl Plugin for WidgetryNumberInputPlugin {
    fn build(&self, app: &mut App) {
        macro_rules! register {
            ($($t:ty),+ $(,)?) => {$(app.register_type::<value::State<$t>>();)+};
        }
        register!(
            i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize, f32, f64
        );
        widgetry_info!("WidgetryNumberInputPlugin 注册完成");
    }
}
