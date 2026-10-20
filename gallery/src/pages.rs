mod button;
mod check_box;
mod combo_box;
mod cursor;
mod list_view;
mod scroll_area;
mod slider;
mod table;
mod text_field;
mod tooltip;
mod tree;
mod waveform;
mod window;

pub(crate) use button::ButtonDemoPlugin;
pub(crate) use check_box::CheckBoxDemoPlugin;
pub(crate) use combo_box::ComboBoxDemoPlugin;
pub(crate) use cursor::CursorDemoPlugin;
pub(crate) use list_view::ListViewDemoPlugin;
pub(crate) use scroll_area::ScrollAreaDemoPlugin;
pub(crate) use slider::SliderDemoPlugin;
pub(crate) use slider::color_examples as slider_color_examples;
pub(crate) use table::TableDemoPlugin;
pub(crate) use text_field::TextFieldDemoPlugin;
pub(crate) use tooltip::TooltipDemoPlugin;
pub(crate) use tree::TreeDemoPlugin;
pub(crate) use waveform::{
    DemoSources as WaveformDemoSources, WaveformDemoPlugin, WaveformDemoState, WaveformDemoSystems,
};
pub(crate) use window::WindowDemoPlugin;

pub(crate) use combo_box::color_examples as combo_color_examples;
pub(crate) use list_view::color_examples as list_color_examples;
pub(crate) use table::color_examples as table_color_examples;
pub(crate) use tree::color_examples as tree_color_examples;
pub(crate) use waveform::color_examples as waveform_color_examples;
