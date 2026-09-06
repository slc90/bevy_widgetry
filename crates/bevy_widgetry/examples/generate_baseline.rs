//! cargo run -p bevy_widgetry --example generate_baseline -- button [output.png]
//! Each invocation renders one widget in one process; use combo_box for ComboBox.
#[path = "../tests/common/mod.rs"]
mod common;
use common::{
    scenes::{button, combo_box},
    visual,
};
fn main() {
    let mut args = std::env::args().skip(1);
    let widget = args.next().expect("expected button or combo_box");
    let (width, height, setup, spawn): (
        _,
        _,
        fn(&mut bevy::prelude::App),
        fn(&mut bevy::prelude::App, bevy::prelude::Entity),
    ) = match widget.as_str() {
        "button" => (button::WIDTH, button::HEIGHT, button::setup, button::spawn),
        "combo_box" => (
            combo_box::WIDTH,
            combo_box::HEIGHT,
            combo_box::setup,
            combo_box::spawn,
        ),
        _ => panic!("expected button or combo_box"),
    };
    let output = args
        .next()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/baselines/styled_{widget}.png"))
        });
    let mut app = visual::setup_offscreen_app();
    setup(&mut app);
    let (target, camera) = visual::create_render_target(&mut app, width, height);
    spawn(&mut app, camera);
    let image = visual::readback_image(&mut app, target, width, height);
    if let Some(parent) = output.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent).unwrap();
    }
    image
        .try_into_dynamic()
        .unwrap()
        .to_rgba8()
        .save(&output)
        .unwrap();
    println!("baseline written to {}", output.display());
}
