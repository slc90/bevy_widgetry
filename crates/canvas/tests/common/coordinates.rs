use bevy::{picking::pointer::Location, prelude::*, ui::UiGlobalTransform};
use bevy_widgetry_canvas::CanvasView;

pub fn at_document(app: &App, view: Entity, base: &Location, point: Vec2) -> Location {
    let camera = app
        .world()
        .get::<bevy::ui::ComputedUiTargetCamera>(view)
        .unwrap()
        .get()
        .unwrap();
    let camera = app.world().get::<Camera>(camera).unwrap();
    let local = (point - CanvasView::center(app.world(), view).unwrap())
        * CanvasView::zoom(app.world(), view).unwrap();
    let node = app.world().get::<ComputedNode>(view).unwrap();
    let physical = app
        .world()
        .get::<UiGlobalTransform>(view)
        .unwrap()
        .transform_point2(local / node.inverse_scale_factor())
        + camera.physical_viewport_rect().unwrap().min.as_vec2();
    Location {
        target: base.target.clone(),
        position: physical / camera.target_scaling_factor().unwrap(),
    }
}
