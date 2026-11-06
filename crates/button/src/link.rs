use crate::link_colors::{LinkColorState, WidgetryLinkButtonColorOverrides};
use crate::link_error::failure;
use crate::open_url::WidgetryOpenUrlState;
use crate::{LinkButtonError, LinkButtonFailure, WidgetryOpenUrl};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{LineBreak, Underline};
use bevy::ui::{InteractionDisabled, Pressed};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::text::WidgetryText;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged};

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryLinkButtonProps)]
#[require(LinkColorState)]
pub struct WidgetryLinkButton;

#[derive(Default, Clone, Debug)]
pub struct WidgetryLinkButtonProps {
    pub url: String,
    pub colors: WidgetryLinkButtonColorOverrides,
}

#[derive(Component)]
struct LinkText;

fn missing(root: Entity, part: &'static str) -> LinkButtonFailure {
    failure(
        LinkButtonError::MissingPart { root, part },
        "link_button.style",
    )
}

fn label(world: &World, root: Entity) -> Result<Entity, LinkButtonFailure> {
    world
        .get::<Children>(root)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<LinkText>(child).is_some())
        })
        .filter(|&child| world.get::<Text>(child).is_some())
        .ok_or_else(|| missing(root, "LinkText.Text"))
}

impl WidgetryLinkButton {
    fn scene(props: WidgetryLinkButtonProps) -> impl Scene {
        let text = props.url.clone();
        bsn! {
            @WidgetryOpenUrl { @url: {props.url.clone()} }
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            Node { width: percent(100), min_width: px(0), flex_shrink: 0.0 }
            template(|_| Ok(ResolvedForeground::default()))
            Children [
                template(|_| Ok(LinkText))
                Text({text.clone()})
                WidgetryText
                TextLayout { linebreak: LineBreak::AnyCharacter }
                Node { width: percent(100), min_width: px(0) }
            ]
        }
    }
}

fn sync_text(world: &mut World) -> Result {
    let roots = world
        .query_filtered::<Entity, (
            With<WidgetryLinkButton>,
            Or<(
                Changed<WidgetryOpenUrlState>,
                Added<WidgetryLinkButton>,
                Changed<Children>,
            )>,
        )>()
        .iter(world)
        .collect::<Vec<_>>();
    let mut errors = bevy_widgetry_diagnostics::bevy::DiagnosticBatch::default();
    for root in roots {
        let result = (|| -> Result<(), LinkButtonFailure> {
            let url = world
                .get::<WidgetryOpenUrlState>(root)
                .ok_or_else(|| missing(root, "WidgetryOpenUrlState"))?
                .url()
                .to_owned();
            let text = label(world, root)?;
            world
                .get_mut::<Text>(text)
                .ok_or_else(|| missing(root, "LinkText.Text"))?
                .set_if_neq(Text(url));
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(error);
        }
    }
    match errors.into_bevy_error() {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

type StyleData = (
    &'static LinkColorState,
    &'static Hovered,
    Has<Pressed>,
    Has<InteractionDisabled>,
    &'static Children,
    &'static mut ResolvedForeground,
);

fn apply_style(
    theme: &bevy_widgetry_theme::WidgetryLinkButtonColors,
    (colors, hovered, pressed, disabled, children, mut foreground): <StyleData as bevy::ecs::query::QueryData>::Item<'_, '_>,
    texts: &Query<Has<Underline>, With<LinkText>>,
    commands: &mut Commands,
) {
    let colors = colors.0.resolve(theme);
    let color = if disabled {
        colors.disabled
    } else if pressed {
        colors.pressed
    } else if hovered.0 {
        colors.hovered
    } else {
        colors.normal
    };
    foreground.set_if_neq(ResolvedForeground(color));
    let underline = !disabled && (hovered.0 || pressed);
    for child in children.iter() {
        if let Ok(current) = texts.get(child)
            && current != underline
        {
            if underline {
                commands.entity(child).insert(Underline);
            } else {
                commands.entity(child).remove::<Underline>();
            }
        }
    }
}

fn update_styles(
    mode: Res<WidgetryTheme>,
    mut roots: Query<
        StyleData,
        (
            With<WidgetryLinkButton>,
            Or<(
                Added<WidgetryLinkButton>,
                Changed<LinkColorState>,
                Changed<Hovered>,
                Added<Pressed>,
                Added<InteractionDisabled>,
            )>,
        ),
    >,
    texts: Query<Has<Underline>, With<LinkText>>,
    mut commands: Commands,
) {
    for item in &mut roots {
        apply_style(&mode.link_button, item, &texts, &mut commands);
    }
}

fn removed_styles(
    mode: Res<WidgetryTheme>,
    mut pressed: RemovedComponents<Pressed>,
    mut disabled: RemovedComponents<InteractionDisabled>,
    mut roots: Query<StyleData, With<WidgetryLinkButton>>,
    texts: Query<Has<Underline>, With<LinkText>>,
    mut commands: Commands,
) {
    for entity in pressed.read().chain(disabled.read()) {
        if let Ok(item) = roots.get_mut(entity) {
            apply_style(&mode.link_button, item, &texts, &mut commands);
        }
    }
}

fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        let mode = *world.resource::<WidgetryTheme>();
        let mut state = bevy::ecs::system::SystemState::<(
            Query<StyleData, With<WidgetryLinkButton>>,
            Query<Has<Underline>, With<LinkText>>,
            Commands,
        )>::new(world);
        let (mut roots, texts, mut commands) = state.get_mut(world).map_err(|error| {
            failure(
                LinkButtonError::ThemeQuery { source: error },
                "link_button.refresh_theme",
            )
            .into_bevy_error()
        })?;
        for item in &mut roots {
            apply_style(&mode.link_button, item, &texts, &mut commands);
        }
        state.apply(world);
        Ok(())
    });
}

pub(crate) fn install(app: &mut App) {
    app.add_systems(PostUpdate, sync_text.in_set(WidgetryUiSystems::Materialize))
        .add_observer(refresh_theme)
        .add_systems(
            PostUpdate,
            (update_styles, removed_styles).in_set(WidgetryUiSystems::Colors),
        );
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{ErrorCapture, scene_app};

    #[test]
    fn damaged_default_text_is_reported_by_style_after_independent_url_commit() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryButtonPlugin);
        app.set_error_handler(ErrorCapture::handler());
        let mut roots = Vec::new();
        for remove_child in [false, true] {
            let root = app
                .world_mut()
                .spawn_scene(bsn! {
                    @WidgetryLinkButton { @url: {"https://example.com".to_owned()} }
                })
                .unwrap()
                .id();
            let text = label(app.world(), root).unwrap();
            if remove_child {
                app.world_mut().despawn(text);
            } else {
                app.world_mut().entity_mut(text).remove::<Text>();
            }
            assert!(
                WidgetryOpenUrl::set_url_in_world(app.world_mut(), root, "https://bevy.org")
                    .unwrap()
            );
            assert_eq!(
                WidgetryOpenUrl::url(app.world(), root).unwrap(),
                "https://bevy.org"
            );
            roots.push(root);
        }
        let errors = ErrorCapture::default();
        errors.run(|| app.update());
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostics = records[0]
            .error
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap();
        let mut reported = Vec::new();
        for error in diagnostics.diagnostics() {
            let source = error
                .source()
                .unwrap()
                .downcast_ref::<LinkButtonError>()
                .unwrap();
            let LinkButtonError::MissingPart {
                root,
                part: "LinkText.Text",
            } = source
            else {
                panic!("wrong style failure");
            };
            reported.push(*root);
        }
        roots.sort();
        reported.sort();
        assert_eq!(reported, roots);
    }

    #[test]
    fn missing_color_state_remains_a_default_style_failure() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryButtonPlugin);
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryLinkButton { @url: {"https://example.com".to_owned()} } })
            .unwrap()
            .id();
        app.world_mut().entity_mut(root).remove::<LinkColorState>();
        assert!(
            matches!(WidgetryLinkButtonColorOverrides::clear_in_world(app.world_mut(), root).unwrap_err().kind(), LinkButtonError::MissingPart { root: target, part: "LinkColorState" } if *target == root)
        );
        assert_eq!(
            WidgetryOpenUrl::url(app.world(), root).unwrap(),
            "https://example.com"
        );
    }
}
