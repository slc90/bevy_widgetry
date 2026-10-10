mod button;
mod check_box;
mod combo_box;
mod list_view;
mod scroll_area;
mod table;
mod text_field;
mod tooltip;
mod tree;
mod waveform;
mod window;

pub(crate) use button::ButtonDemoPlugin;
pub(crate) use check_box::{CheckBoxDemoPlugin, scene as check_box};
pub(crate) use combo_box::{ComboBoxDemoPlugin, ComboBoxDemoSources, scene as combo_box};
pub(crate) use list_view::{
    DemoSources as ListViewDemoSources, ListViewDemoPlugin, scene as list_view,
};
pub(crate) use scroll_area::scene as scroll_area;
pub(crate) use table::{TableDemoPlugin, TableDemoSources, scene as table};
pub(crate) use text_field::scene as text_field;
pub(crate) use tooltip::scene as tooltip;
pub(crate) use tree::{DemoSources as TreeDemoSources, TreeDemoPlugin, scene as tree};
pub(crate) use waveform::{
    DemoSources as WaveformDemoSources, WaveformDemoPlugin, WaveformDemoState, WaveformDemoSystems,
    scene as waveform,
};
pub(crate) use window::{WindowDemoPlugin, scene as window};

pub(crate) use combo_box::color_examples as combo_color_examples;
pub(crate) use list_view::color_examples as list_color_examples;
pub(crate) use table::color_examples as table_color_examples;
pub(crate) use tree::color_examples as tree_color_examples;
pub(crate) use waveform::color_examples as waveform_color_examples;
