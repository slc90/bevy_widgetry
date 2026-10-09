mod colors;
pub use colors::{WidgetryTextColorOverrides, WidgetryTextStateColorOverrides};

use crate::{foreground::InheritedForeground, ui::WidgetryUiSystems};
use bevy::prelude::*;
use bevy::text::TextSpan;
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Component, FromTemplate, Default, Clone)]
#[require(colors::ColorState)]
pub struct WidgetryText;

#[derive(Default, Clone, Debug)]
pub struct WidgetryTextProps {
    pub colors: WidgetryTextColorOverrides,
}

// 纯颜色标记也支持直接添加到已有 Text，不能要求它必须随 Scene 一起构造。
impl SceneComponent for WidgetryText {
    type Props = WidgetryTextProps;

    fn scene(props: Self::Props) -> impl Scene {
        bsn! { WidgetryText template(move |_| props.colors.clone().initial()) }
    }
}

impl WidgetryText {
    pub fn set_color_in_world(
        world: &mut World,
        entity: Entity,
        color: Color,
    ) -> Result<bool, BevyError> {
        WidgetryTextColorOverrides::set_in_world(
            world,
            entity,
            WidgetryTextColorOverrides {
                normal: WidgetryTextStateColorOverrides {
                    foreground: Some(color),
                },
                disabled: WidgetryTextStateColorOverrides {
                    foreground: Some(color),
                },
            },
        )
    }

    pub fn clear_color_in_world(world: &mut World, entity: Entity) -> Result<bool, BevyError> {
        WidgetryTextColorOverrides::clear_in_world(world, entity)
    }
    pub fn set_color(commands: &mut Commands, entity: Entity, color: Color) {
        commands.queue(move |world: &mut World| {
            Self::set_color_in_world(world, entity, color).map(|_| ())
        });
    }
    pub fn clear_color(commands: &mut Commands, entity: Entity) {
        WidgetryTextColorOverrides::clear(commands, entity);
    }
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(
        PostUpdate,
        apply_text_colors.in_set(WidgetryUiSystems::ContentColors),
    );
}

fn apply_text_colors(
    mode: Res<WidgetryThemeMode>,
    mut query: Query<
        (
            &colors::ColorState,
            Option<&InheritedForeground>,
            &crate::foreground::ContentContext,
            &mut TextColor,
        ),
        (
            With<WidgetryText>,
            Without<bevy::text::EditableText>,
            Or<(With<Text>, With<TextSpan>)>,
        ),
    >,
) {
    for (overrides, inherited, context, mut output) in &mut query {
        if !context.ui_text {
            continue;
        }
        let disabled = context.disabled;
        let resolved = overrides.0.resolve(&mode.colors().text);
        let (local, theme) = if disabled {
            (&overrides.0.disabled, resolved.disabled)
        } else {
            (&overrides.0.normal, resolved.normal)
        };
        let color = local
            .foreground
            .or_else(|| {
                inherited
                    .filter(|source| source.disabled == disabled)
                    .map(|source| source.color)
            })
            .unwrap_or(theme.foreground);
        output.set_if_neq(TextColor(color));
    }
}
