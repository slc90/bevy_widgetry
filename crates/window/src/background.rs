use bevy::prelude::*;
use bevy::ui::VisualBox;
use bevy_widgetry_core::ThemeMode;

#[derive(Component)]
pub(crate) struct ThemeWindowBackground;

#[derive(Clone, Debug)]
pub struct WidgetryWindowImageBackground {
    pub image: Handle<Image>,
    pub mode: WidgetryWindowImageMode,
    pub opacity: f32,
}

#[derive(Clone, Debug)]
pub enum WidgetryWindowBackground {
    Theme,
    Image(WidgetryWindowImageBackground),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryWindowImageMode {
    Stretch,
    Cover,
}

pub(crate) fn window_background(background: WidgetryWindowBackground) -> impl Scene {
    let theme = matches!(background, WidgetryWindowBackground::Theme);
    let image = match background {
        WidgetryWindowBackground::Theme => None,
        WidgetryWindowBackground::Image(config) => Some(config),
    };
    bsn! {
        {theme.then(|| bsn! {
            template(|_| Ok(ThemeWindowBackground))
            template(|context| Ok(BackgroundColor(context.resource::<ThemeMode>().colors().window_background)))
        })}
        {image.map(|config| bsn! {
            template(move |_| {
                Ok(ImageNode {
                    image: config.image.clone(),
                    image_mode: NodeImageMode::Stretch,
                    color: Color::srgba(1.0, 1.0, 1.0, config.opacity),
                    visual_box: VisualBox::BorderBox,
                    ..default()
                })
            })
        })}
    }
}
