//! 向 Gallery 外壳提供各 Widget 的演示 Scene，隐藏具体页面 module。

mod button;
mod check_box;
mod combo_box;
mod text_field;
mod tooltip;
mod window;

pub(crate) use button::{ButtonDemoPlugin, scene as button};
pub(crate) use check_box::{CheckBoxDemoPlugin, scene as check_box};
pub(crate) use combo_box::scene as combo_box;
pub(crate) use text_field::scene as text_field;
pub(crate) use tooltip::{TooltipDemoPlugin, scene as tooltip};
pub(crate) use window::{WindowDemoPlugin, scene as window};
