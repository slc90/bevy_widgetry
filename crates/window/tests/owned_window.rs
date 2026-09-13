use bevy::{
    camera::RenderTarget,
    prelude::*,
    window::{WindowClosed, WindowRef},
};
use bevy_widgetry_test_utils::scene_app;
use bevy_widgetry_window::{WindowControlsConfig, WindowPlugin, owned_window};

/// 无桌面环境下验证 owned 场景创建独立资源并正确绑定 UI 相机。
#[test]
fn owned_resources_follow_root_lifetime() {
    for native_first in [false, true] {
        let mut app = scene_app();
        app.add_plugins(WindowPlugin);
        let root = app.world_mut().commands().spawn_scene(bsn! {
            owned_window(Window::default(), WindowControlsConfig::default(), bsn_list![], bsn_list![])
        }).id();
        app.update();
        let camera = app.world().get::<UiTargetCamera>(root).unwrap().0;
        let target = app
            .world_mut()
            .query_filtered::<Entity, With<Window>>()
            .single(app.world())
            .unwrap();
        assert!(
            matches!(app.world().get::<RenderTarget>(camera), Some(RenderTarget::Window(WindowRef::Entity(entity))) if *entity == target)
        );
        if native_first {
            app.world_mut().entity_mut(target).despawn();
            app.world_mut()
                .write_message(WindowClosed { window: target });
            app.update();
        } else {
            app.world_mut().entity_mut(root).despawn();
            app.world_mut().flush();
        }
        for entity in [root, target, camera] {
            assert!(app.world().get_entity(entity).is_err());
        }
    }
}
