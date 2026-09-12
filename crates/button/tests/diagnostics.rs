use bevy::{
    ecs::schedule::SingleThreadedExecutor, prelude::*, text::FontSource, ui_widgets::Button,
};
use bevy_widgetry_button::{LongPressButton, LongPressPlugin, StyledButton, StyledButtonPlugin};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_test_utils::LogCapture;

// 已确认按钮缺少必需组件时报告一次，恢复后记录一次；构造及销毁保持安静。
#[test]
fn button_required_components_log_edges() {
    for styled in [false, true] {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins);
            app.set_default_font(FontSource::Monospace);
            app.add_plugins((LongPressPlugin, StyledButtonPlugin))
                .edit_schedule(PostUpdate, |schedule| {
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
            let entity = if styled {
                app.world_mut().spawn(StyledButton).id()
            } else {
                app.world_mut().spawn(LongPressButton::default()).id()
            };
            app.update();
            assert!(
                capture
                    .records()
                    .iter()
                    .all(|record| record.level == bevy::log::Level::INFO)
            );
            let baseline = capture.records().len();
            if styled {
                app.world_mut().entity_mut(entity).remove::<BorderColor>();
            } else {
                app.world_mut().entity_mut(entity).remove::<Button>();
            }
            app.update();
            app.update();
            assert_eq!(capture.records().len(), baseline + 1);
            assert_eq!(capture.records()[baseline].level, bevy::log::Level::ERROR);
            if styled {
                app.world_mut()
                    .entity_mut(entity)
                    .insert(BorderColor::default());
            } else {
                app.world_mut().entity_mut(entity).insert(Button);
            }
            app.update();
            app.update();
            assert_eq!(capture.records().len(), baseline + 2);
            assert!(capture.records()[baseline + 1].fields["message"].contains("恢复"));
            if styled {
                app.world_mut().entity_mut(entity).remove::<BorderColor>();
            } else {
                app.world_mut().entity_mut(entity).remove::<Button>();
            }
            app.update();
            app.update();
            assert_eq!(capture.records().len(), baseline + 3);
            assert_eq!(
                capture.records()[baseline + 2].level,
                bevy::log::Level::ERROR
            );
            app.world_mut().entity_mut(entity).despawn();
            app.update();
            assert_eq!(capture.records().len(), baseline + 3);
        });
    }
}
