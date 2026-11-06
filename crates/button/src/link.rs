use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{LineBreak, Underline};
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::{Activate, Button};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::text::WidgetryText;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_theme::{WidgetryTheme, WidgetryThemeChanged};

use crate::link_colors::{LinkColorState, WidgetryLinkButtonColorOverrides};
use crate::link_error::failure;
use crate::{LinkButtonError, LinkButtonFailure, LinkUrlRejection};

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
struct LinkUrl(String);

#[derive(Component)]
struct LinkText;

#[derive(Resource)]
struct BrowserOpener(fn(Entity, &str) -> Result<(), LinkButtonFailure>);

impl Default for BrowserOpener {
    fn default() -> Self {
        Self(open_browser)
    }
}

fn open_browser(root: Entity, url: &str) -> Result<(), LinkButtonFailure> {
    winsafe::ShellExecuteEx(&winsafe::SHELLEXECUTEINFO {
        mask: winsafe::co::SEE_MASK::FLAG_NO_UI | winsafe::co::SEE_MASK::NOASYNC,
        verb: Some("open"),
        file: url,
        show: winsafe::co::SW::SHOWNORMAL,
        ..default()
    })
    .map_err(|source| {
        failure(
            LinkButtonError::OpenBrowser {
                root,
                url: url.to_owned(),
                source,
            },
            "link_button.open_browser",
        )
    })
}

fn validate_url(root: Entity, raw: &str) -> Result<(), LinkButtonFailure> {
    let invalid = |reason| {
        failure(
            LinkButtonError::InvalidUrl {
                root,
                url: raw.to_owned(),
                reason,
            },
            "link_button.validate_url",
        )
    };
    // The URL parser repairs missing slashes and strips controls, whereas Shell receives the original string.
    // Reject these inputs before parsing so validation and the native request agree.
    if raw.chars().any(|ch| ch.is_control() || ch.is_whitespace()) || raw.contains('\\') {
        return Err(invalid(LinkUrlRejection::UnsafeCharacters));
    }
    let Some((scheme, authority)) = raw.split_once("://") else {
        return Err(invalid(LinkUrlRejection::Incomplete));
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")) {
        return Err(invalid(LinkUrlRejection::UnsupportedScheme));
    }
    if authority.is_empty() || authority.starts_with(['/', '?', '#']) {
        return Err(invalid(LinkUrlRejection::MissingHost));
    }
    let violation = std::cell::Cell::new(None);
    let check_syntax = |syntax| {
        if syntax != url::SyntaxViolation::EmbeddedCredentials {
            violation.set(Some(syntax));
        }
    };
    let parsed = url::Url::options()
        .syntax_violation_callback(Some(&check_syntax))
        .parse(raw)
        .map_err(|source| invalid(LinkUrlRejection::Parse(source)))?;
    if let Some(violation) = violation.get() {
        return Err(invalid(LinkUrlRejection::Syntax(violation)));
    }
    if !parsed.has_host() {
        return Err(invalid(LinkUrlRejection::MissingHost));
    }
    Ok(())
}

fn invalid_target(root: Entity) -> LinkButtonFailure {
    failure(
        LinkButtonError::InvalidTarget { root },
        "link_button.target",
    )
}

fn missing(root: Entity, part: &'static str) -> LinkButtonFailure {
    failure(
        LinkButtonError::MissingPart { root, part },
        "link_button.parts",
    )
}

fn label(world: &World, entity: Entity) -> Result<Entity, LinkButtonFailure> {
    if world.get::<WidgetryLinkButton>(entity).is_none() {
        return Err(invalid_target(entity));
    }
    world
        .get::<Children>(entity)
        .and_then(|children| {
            children
                .iter()
                .find(|&child| world.get::<LinkText>(child).is_some())
        })
        .filter(|&child| world.get::<Text>(child).is_some())
        .ok_or_else(|| missing(entity, "LinkText.Text"))
}

impl WidgetryLinkButton {
    pub fn url(world: &World, entity: Entity) -> Result<&str, LinkButtonFailure> {
        if world.get::<Self>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<LinkUrl>(entity)
            .map(|url| url.0.as_str())
            .ok_or_else(|| missing(entity, "LinkUrl"))
    }

    pub fn set_url_in_world(
        world: &mut World,
        entity: Entity,
        url: impl Into<String>,
    ) -> Result<bool, LinkButtonFailure> {
        let url = url.into();
        let current = Self::url(world, entity)?;
        let text_entity = label(world, entity)?;
        if current == url {
            return Ok(false);
        }
        validate_url(entity, &url)?;
        let mut text = world
            .get_mut::<Text>(text_entity)
            .ok_or_else(|| missing(entity, "LinkText.Text"))?;
        text.0.clone_from(&url);
        let mut state = world
            .get_mut::<LinkUrl>(entity)
            .ok_or_else(|| missing(entity, "LinkUrl"))?;
        state.0 = url;
        Ok(true)
    }

    pub fn set_url(commands: &mut Commands, entity: Entity, url: impl Into<String>) {
        let url = url.into();
        commands.queue(move |world: &mut World| {
            Self::set_url_in_world(world, entity, url)
                .map(|_| ())
                .map_err(|error| error.into_bevy_error())
        });
    }

    fn scene(props: WidgetryLinkButtonProps) -> impl Scene {
        let text = props.url.clone();
        bsn! {
            template(move |context| {
                validate_url(context.entity.id(), &props.url).map_err(|error| error.into_bevy_error())?;
                Ok(LinkUrl(props.url.clone()))
            })
            template(move |context| props.colors.clone().initial(context.entity.id()).map_err(|error| error.into_bevy_error()))
            Button
            template(|_| Ok(bevy::picking::cursor::EntityCursor::System(bevy::window::SystemCursorIcon::Pointer)))
            bevy_widgetry_core::pointer::WidgetryPointerPressed
            Hovered(false)
            TabIndex(-1)
            Node { width: percent(100), min_width: px(0), flex_shrink: 0.0 }
            template(|_| Ok(ResolvedForeground::default()))
            on(activate)
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

fn activate(event: On<Activate>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        // Check at execution time too, because a preceding queued command may disable or remove the link.
        if world.get::<WidgetryLinkButton>(entity).is_none()
            || world.get::<InteractionDisabled>(entity).is_some()
        {
            return Ok(());
        }
        let url =
            WidgetryLinkButton::url(world, entity).map_err(|error| error.into_bevy_error())?;
        if let Err(error) = (world.resource::<BrowserOpener>().0)(entity, url) {
            error.record_handled("放弃本次系统浏览器打开请求", "请求已终止，不重试");
        }
        Ok(())
    });
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
    app.init_resource::<BrowserOpener>()
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
    use bevy_widgetry_test_utils::{
        DiagnosticPhase, ErrorCapture, LogCapture, press, primary_click, release, scene_app,
    };
    use std::cell::RefCell;
    use std::error::Error;

    thread_local! {
        static REQUESTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static ORIGINS: RefCell<Vec<std::sync::Arc<bevy_widgetry_diagnostics::ErrorOrigin>>> = const { RefCell::new(Vec::new()) };
    }

    fn fake_browser(root: Entity, url: &str) -> Result<(), LinkButtonFailure> {
        REQUESTS.with_borrow_mut(|requests| requests.push(url.to_owned()));
        let error = failure(
            LinkButtonError::OpenBrowser {
                root,
                url: url.to_owned(),
                source: winsafe::co::ERROR::ACCESS_DENIED,
            },
            "test.open_browser",
        );
        assert_eq!(
            *error
                .kind()
                .source()
                .unwrap()
                .downcast_ref::<winsafe::co::ERROR>()
                .unwrap(),
            winsafe::co::ERROR::ACCESS_DENIED
        );
        ORIGINS.with_borrow_mut(|origins| origins.push(error.origin().clone()));
        Err(error)
    }

    #[test]
    fn activation_requests_current_url_once_and_handles_native_failure_with_original_identity() {
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| {
            let mut app = scene_app();
            app.add_plugins(crate::WidgetryButtonPlugin);
            app.set_error_handler(ErrorCapture::handler());
            app.insert_resource(BrowserOpener(fake_browser));
            let parent = app.world_mut().spawn_scene(bsn! { Node Children [@WidgetryLinkButton { @url: {"https://www.baidu.com".to_owned()} }] }).unwrap().id();
            let root = app.world().get::<Children>(parent).unwrap()[0];
            app.update();
            REQUESTS.with_borrow_mut(Vec::clear);
            ORIGINS.with_borrow_mut(Vec::clear);
            errors.run(|| {
            WidgetryLinkButton::set_url_in_world(app.world_mut(), root, "https://www.google.com").unwrap();
            app.update();
            REQUESTS.with_borrow(|requests| assert!(requests.is_empty()));
            press(&mut app, root);
            app.world_mut().trigger(primary_click(root));
            release(&mut app, root);
            app.update();
            REQUESTS.with_borrow(|requests| assert_eq!(requests.as_slice(), ["https://www.google.com"]));
            app.world_mut().entity_mut(parent).insert(InteractionDisabled);
            app.world_mut().flush();
            press(&mut app, root);
            app.world_mut().trigger(primary_click(root));
            app.world_mut().trigger(Activate { entity: root });
            release(&mut app, root);
            app.update();
            REQUESTS.with_borrow(|requests| assert_eq!(requests.len(), 1));
            app.world_mut().entity_mut(parent).remove::<InteractionDisabled>();
            app.world_mut().flush();
            app.world_mut().trigger(Activate { entity: root });
            app.world_mut().flush();
            REQUESTS.with_borrow(|requests| assert_eq!(requests.len(), 2));
            });
        });
        assert!(errors.take_records().is_empty());
        ORIGINS.with_borrow(|origins| {
            assert_eq!(origins.len(), 2);
            assert_ne!(origins[0].id(), origins[1].id());
            for origin in origins {
                assert_eq!(
                    logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                        .len(),
                    1
                );
                let handled = logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled);
                assert_eq!(handled.len(), 1);
                assert!(
                    handled[0]
                        .fields
                        .get("action")
                        .unwrap()
                        .contains("放弃本次系统浏览器打开请求")
                );
            }
        });
        assert!(
            !logs
                .records()
                .iter()
                .any(|record| record.level == bevy::log::Level::WARN)
        );
    }

    fn successful_browser(_: Entity, url: &str) -> Result<(), LinkButtonFailure> {
        REQUESTS.with_borrow_mut(|requests| requests.push(url.to_owned()));
        Ok(())
    }

    #[test]
    fn successful_requests_and_queued_disabled_or_despawned_cancellations_are_silent() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryButtonPlugin);
        app.insert_resource(BrowserOpener(successful_browser));
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryLinkButton { @url: {"https://example.com".to_owned()} } })
            .unwrap()
            .id();
        REQUESTS.with_borrow_mut(Vec::clear);
        let logs = LogCapture::default();
        logs.run(|| {
            app.world_mut().trigger(Activate { entity: root });
            app.world_mut().flush();
            REQUESTS
                .with_borrow(|requests| assert_eq!(requests.as_slice(), ["https://example.com"]));
            app.world_mut().commands().queue(move |world: &mut World| {
                world.entity_mut(root).insert(InteractionDisabled);
            });
            app.world_mut().trigger(Activate { entity: root });
            app.world_mut().flush();
            app.world_mut().commands().queue(move |world: &mut World| {
                world.despawn(root);
            });
            app.world_mut().trigger(Activate { entity: root });
            app.world_mut().flush();
            REQUESTS.with_borrow(|requests| assert_eq!(requests.len(), 1));
            assert!(logs.records().is_empty());
        });
    }

    #[test]
    fn damaged_link_parts_are_distinct_from_invalid_targets_and_reject_before_commit() {
        for part in ["LinkUrl", "LinkText.Text", "LinkColorState"] {
            let mut app = scene_app();
            app.add_plugins(crate::WidgetryButtonPlugin);
            let root = app
                .world_mut()
                .spawn_scene(
                    bsn! { @WidgetryLinkButton { @url: {"https://example.com".to_owned()} } },
                )
                .unwrap()
                .id();
            let text = label(app.world(), root).unwrap();
            let error = match part {
                "LinkUrl" => {
                    app.world_mut().entity_mut(root).remove::<LinkUrl>();
                    WidgetryLinkButton::set_url_in_world(app.world_mut(), root, "https://bevy.org")
                        .unwrap_err()
                }
                "LinkText.Text" => {
                    app.world_mut().entity_mut(text).remove::<Text>();
                    WidgetryLinkButton::set_url_in_world(app.world_mut(), root, "https://bevy.org")
                        .unwrap_err()
                }
                _ => {
                    app.world_mut().entity_mut(root).remove::<LinkColorState>();
                    WidgetryLinkButtonColorOverrides::clear_in_world(app.world_mut(), root)
                        .unwrap_err()
                }
            };
            assert!(
                matches!(error.kind(), LinkButtonError::MissingPart { root: target, part: actual } if *target == root && *actual == part)
            );
            if let Some(url) = app.world().get::<LinkUrl>(root) {
                assert_eq!(url.0, "https://example.com");
            }
            if let Some(text) = app.world().get::<Text>(text) {
                assert_eq!(text.0, "https://example.com");
            }
        }
    }
}
