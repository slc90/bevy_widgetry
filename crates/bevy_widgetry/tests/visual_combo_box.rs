#[path = "common/scenes/combo_box.rs"]
mod scene;
#[path = "common/visual.rs"]
mod visual;
#[test]
fn styled_combo_box_can_render_offscreen() {
    let mut app = visual::setup_offscreen_app();
    scene::setup(&mut app);
    let (target, camera) = visual::create_render_target(&mut app, scene::WIDTH, scene::HEIGHT);
    scene::spawn(&mut app, camera);
    let image = visual::readback_image(&mut app, target, scene::WIDTH, scene::HEIGHT);
    visual::assert_matches_baseline(image, "styled_combo_box.png");
}
