#![cfg(feature = "number_input")]
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]
use bevy::prelude::*;
use bevy_widgetry::{
    number_input::{WidgetryNumberInput, WidgetryNumberInputPlugin},
    scene::spawn_scene,
};

#[test]
fn facade_exposes_typed_number_input_and_deferred_value_api() {
    let mut app = bevy_widgetry_test_utils::scene_app();
    app.add_plugins(WidgetryNumberInputPlugin);
    let root = spawn_scene(
        app.world_mut(),
        bsn! { @WidgetryNumberInput<u64> { @value: {Some(u64::MAX-2)} } },
    )
    .unwrap();
    WidgetryNumberInput::<u64>::set_value(&mut app.world_mut().commands(), root, u64::MAX);
    assert_eq!(
        WidgetryNumberInput::<u64>::value(app.world(), root).unwrap(),
        u64::MAX - 2
    );
    app.world_mut().flush();
    assert_eq!(
        WidgetryNumberInput::<u64>::value(app.world(), root).unwrap(),
        u64::MAX
    );
}
