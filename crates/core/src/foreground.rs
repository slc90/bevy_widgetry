use bevy::{
    app::{App, HierarchyPropagatePlugin, Plugin, PostUpdate, PropagateSet},
    color::Color,
    ecs::{component::Component, query::Changed, schedule::IntoScheduleConfigs, system::Query},
    text::TextColor,
    ui::UiSystems,
};

/// 可沿实体层级传播的前景色；配合 ForegroundColorPlugin 同步 TextColor，默认黑色。
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ForegroundColor(pub Color);

/// 在 PostUpdate 沿层级传播前景色，并同步到文本颜色。
pub struct ForegroundColorPlugin;

/// 只在前景色发生变化时覆盖文本颜色，供层级传播结束后调用。
fn apply_foreground_color_to_text(
    mut query: Query<(&ForegroundColor, &mut TextColor), Changed<ForegroundColor>>,
) {
    for (foreground, mut text_color) in &mut query {
        text_color.0 = foreground.0;
    }
}

impl Default for ForegroundColor {
    fn default() -> Self {
        Self(Color::BLACK)
    }
}

impl Plugin for ForegroundColorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HierarchyPropagatePlugin::<ForegroundColor>::new(PostUpdate));
        app.configure_sets(
            PostUpdate,
            PropagateSet::<ForegroundColor>::default().in_set(UiSystems::Propagate),
        );
        app.add_systems(
            PostUpdate,
            apply_foreground_color_to_text
                .in_set(UiSystems::Propagate)
                .after(PropagateSet::<ForegroundColor>::default()),
        );
    }
}
