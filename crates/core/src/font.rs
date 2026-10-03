use crate::ui::{WidgetryUiPlugin, WidgetryUiSystems};
use bevy::prelude::*;
use bevy::text::{FontSource, detect_text_needs_rerender};
use bevy::ui::UiSystems;
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};
use bevy_widgetry_log::widgetry_info;

#[derive(Resource)]
struct DefaultFont(FontSource);

pub struct WidgetryFontPlugin;

pub trait WidgetryAppExt {
    fn set_default_font(&mut self, font: FontSource) -> &mut Self;
}

fn apply_default_font(
    default_font: Res<DefaultFont>,
    mut fonts: Query<&mut TextFont, Added<TextFont>>,
) {
    for mut text_font in &mut fonts {
        if text_font.font == FontSource::default() {
            text_font.font = default_font.0.clone();
        }
    }
}

impl WidgetryAppExt for App {
    fn set_default_font(&mut self, font: FontSource) -> &mut Self {
        self.insert_resource(DefaultFont(font));
        if !self.is_plugin_added::<WidgetryFontPlugin>() {
            self.add_plugins(WidgetryFontPlugin);
        }
        self
    }
}

impl Plugin for WidgetryFontPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<WidgetryUiPlugin>() {
            app.add_plugins(WidgetryUiPlugin);
        }
        if !app.world().contains_resource::<DefaultFont>() {
            if !app.is_plugin_added::<WidgetryAssetPlugin>() {
                app.add_plugins(WidgetryAssetPlugin);
            }
            let font = app
                .world()
                .resource::<AssetServer>()
                .load(BuiltinFont::Default.path());
            app.insert_resource(DefaultFont(FontSource::Handle(font)));
        }
        app.add_systems(
            PostUpdate,
            apply_default_font
                .after(WidgetryUiSystems::Materialize)
                .after(UiSystems::Prepare)
                .before(detect_text_needs_rerender),
        );
        widgetry_info!("WidgetryFontPlugin 注册完成");
    }
}

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::NodeId;

    #[test]
    fn fallback_precedes_bevy_text_detection() {
        let mut app = App::new();
        app.set_default_font(FontSource::Monospace);
        app.world_mut()
            .schedule_scope(PostUpdate, |world, schedule| {
                schedule.graph_mut().initialize(world);
                let graph = schedule.graph();
                let fallback = graph
                    .systems
                    .iter()
                    .find_map(|(key, system, _)| {
                        (system.system_type()
                            == IntoSystem::into_system(apply_default_font).system_type())
                        .then_some(NodeId::System(key))
                    })
                    .unwrap();
                let detection_sets =
                    IntoSystem::into_system(detect_text_needs_rerender).default_system_sets();
                let detection = graph
                    .system_sets
                    .iter()
                    .find_map(|(key, set, _)| {
                        detection_sets
                            .iter()
                            .any(|expected| set == expected.0)
                            .then_some(NodeId::Set(key))
                    })
                    .unwrap();
                let prepare = graph
                    .system_sets
                    .iter()
                    .find_map(|(key, set, _)| {
                        (set == &*UiSystems::Prepare.intern()).then_some(NodeId::Set(key))
                    })
                    .expect("默认字体应明确等待 UI Prepare，不能依赖碰巧的 system 顺序");
                assert!(graph.dependency().graph().contains_edge(prepare, fallback));
                assert!(
                    graph
                        .dependency()
                        .graph()
                        .contains_edge(fallback, detection)
                );
            });
    }
}
