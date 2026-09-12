use bevy::{
    ecs::schedule::SingleThreadedExecutor,
    input_focus::InputFocus,
    prelude::*,
    text::{EditableText, FontSource},
};
use bevy_widgetry_core::WidgetryAppExt;
use bevy_widgetry_test_utils::LogCapture;
use bevy_widgetry_text_field::{StyledTextField, StyledTextFieldPlugin, TextField};

// 编辑组件与样式组件缺失均不能被 Query 过滤静默隐藏；持续异常与恢复各记录一次。
#[test]
fn text_field_required_components_log_edges() {
    for styled in [false, true] {
        let capture = LogCapture::default();
        capture.run(|| {
            let mut app = App::new();
            app.set_default_font(FontSource::Monospace);
            app.init_resource::<InputFocus>()
                .add_plugins(StyledTextFieldPlugin)
                .edit_schedule(PostUpdate, |schedule| {
                    schedule.set_executor(SingleThreadedExecutor::new());
                });
            let entity = if styled {
                app.world_mut().spawn(StyledTextField).id()
            } else {
                app.world_mut().spawn(TextField).id()
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
                app.world_mut().entity_mut(entity).remove::<EditableText>();
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
                app.world_mut()
                    .entity_mut(entity)
                    .insert(EditableText::default());
            }
            app.update();
            app.update();
            assert_eq!(capture.records().len(), baseline + 2);
            assert!(capture.records()[baseline + 1].fields["message"].contains("恢复"));
            if styled {
                app.world_mut().entity_mut(entity).remove::<BorderColor>();
            } else {
                app.world_mut().entity_mut(entity).remove::<EditableText>();
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
