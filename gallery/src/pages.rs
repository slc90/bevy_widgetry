//! 向 Gallery 外壳提供各控件的演示场景，隐藏具体页面模块。

mod button;
mod combo_box;
mod text_field;
mod window;

pub(crate) use button::scene as button;
pub(crate) use combo_box::scene as combo_box;
pub(crate) use text_field::scene as text_field;
pub(crate) use window::{WindowDemoPlugin, scene as window};
