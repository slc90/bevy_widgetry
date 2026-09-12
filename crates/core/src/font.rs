use bevy::prelude::*;
use bevy::text::{FontSource, detect_text_needs_rerender};
use bevy_widgetry_asset::{BuiltinFont, WidgetryAssetPlugin};

/// 初始化阶段确定的 App fallback，不用于运行时统一替换既有文本。
#[derive(Resource)]
struct DefaultFont(FontSource);

/// Widgetry 内部共享字体设施，由样式插件或 WidgetryAppExt 自动注册。
/// 使用内建字体时须先注册 Bevy 的资产与文本插件（通常为 DefaultPlugins）。
pub struct WidgetryFontPlugin;

/// 配置整个 App 的默认字体，包括普通 Bevy 文本。
pub trait WidgetryAppExt {
    /// 在首次 update / run 前设置 fallback，并自动启用字体插件。
    ///
    /// 仅新加入 ECS 且 font 等于 FontSource::default() 的 TextFont 会被替换；
    /// 即使显式写入该哨兵值也会使用 fallback，其他显式字体保持不变。
    /// 可在 Widgetry 样式插件之前或之后调用；未配置时样式插件使用内建得意黑。
    /// 系统字体及通用字体族的解析遵循 Bevy 配置，自有字体可传 FontSource::Handle。
    /// 不支持通过此方法在运行时统一切换既有文本的字体。
    fn set_default_font(&mut self, font: FontSource) -> &mut Self;
}

/// 在 Bevy 检测文本变更前，仅为本次新加入的默认字体填入 App 配置。
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
            apply_default_font.before(detect_text_needs_rerender),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::NodeId;

    // 检查调度依赖边，防止系统恰巧先执行而掩盖本帧文字测量使用旧字体的回归。
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
                assert!(
                    graph
                        .dependency()
                        .graph()
                        .contains_edge(fallback, detection)
                );
            });
    }
}
