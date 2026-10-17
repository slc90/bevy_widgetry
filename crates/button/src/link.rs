use bevy::input_focus::tab_navigation::TabIndex;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::{LineBreak, Underline};
use bevy::ui::{InteractionDisabled, Pressed};
use bevy::ui_widgets::{Activate, Button};
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_core::text::WidgetryText;
use bevy_widgetry_core::ui::WidgetryUiSystems;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_warn};
use bevy_widgetry_theme::{WidgetryThemeChanged, WidgetryThemeMode};

use crate::link_colors::{LinkColorState, WidgetryLinkButtonColorOverrides};

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
struct BrowserOpener(fn(&str) -> Result<(), String>);

impl Default for BrowserOpener {
    fn default() -> Self {
        Self(open_browser)
    }
}

fn open_browser(url: &str) -> Result<(), String> {
    winsafe::ShellExecuteEx(&winsafe::SHELLEXECUTEINFO {
        mask: winsafe::co::SEE_MASK::FLAG_NO_UI | winsafe::co::SEE_MASK::NOASYNC,
        verb: Some("open"),
        file: url,
        show: winsafe::co::SW::SHOWNORMAL,
        ..default()
    })
    .map_err(|error| error.to_string())
}

fn validate_url(raw: &str) -> Result<(), BevyError> {
    let invalid = |reason: &str| {
        widgetry_error!(url = raw, reason, "LinkButton URL 无效");
        BevyError::error(format!("LinkButton URL 无效: {reason}"))
    };
    // The URL parser repairs missing slashes and strips controls, whereas Shell receives the original string.
    // Reject these inputs before parsing so validation and the native request agree.
    if raw.chars().any(|ch| ch.is_control() || ch.is_whitespace()) || raw.contains('\\') {
        return Err(invalid("URL 不能包含空白、控制字符或反斜杠"));
    }
    let Some((scheme, authority)) = raw.split_once("://") else {
        return Err(invalid("必须提供完整 HTTP / HTTPS URL"));
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        || authority.is_empty()
        || authority.starts_with(['/', '?', '#'])
    {
        return Err(invalid("仅支持带主机的 HTTP / HTTPS URL"));
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
        .map_err(|error| invalid(&error.to_string()))?;
    if let Some(violation) = violation.get() {
        return Err(invalid(violation.description()));
    }
    if !parsed.has_host() {
        return Err(invalid("URL 缺少主机"));
    }
    Ok(())
}

fn invalid_target(entity: Entity) -> BevyError {
    widgetry_error!(?entity, "LinkButton 目标不存在、类型不匹配或内部文字缺失");
    BevyError::error("LinkButton 目标不存在、类型不匹配或内部文字缺失")
}

fn label(world: &World, entity: Entity) -> Result<Entity, BevyError> {
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
        .ok_or_else(|| invalid_target(entity))
}

impl WidgetryLinkButton {
    pub fn url(world: &World, entity: Entity) -> Result<&str, BevyError> {
        if world.get::<Self>(entity).is_none() {
            return Err(invalid_target(entity));
        }
        world
            .get::<LinkUrl>(entity)
            .map(|url| url.0.as_str())
            .ok_or_else(|| invalid_target(entity))
    }

    pub fn set_url_in_world(
        world: &mut World,
        entity: Entity,
        url: impl Into<String>,
    ) -> Result<bool, BevyError> {
        let url = url.into();
        let current = Self::url(world, entity)?;
        let text_entity = label(world, entity)?;
        if current == url {
            return Ok(false);
        }
        validate_url(&url)?;
        let mut text = world
            .get_mut::<Text>(text_entity)
            .ok_or_else(|| invalid_target(entity))?;
        text.0.clone_from(&url);
        let mut state = world
            .get_mut::<LinkUrl>(entity)
            .ok_or_else(|| invalid_target(entity))?;
        state.0 = url;
        Ok(true)
    }

    pub fn set_url(commands: &mut Commands, entity: Entity, url: impl Into<String>) {
        let url = url.into();
        commands
            .queue(move |world: &mut World| Self::set_url_in_world(world, entity, url).map(|_| ()));
    }

    fn scene(props: WidgetryLinkButtonProps) -> impl Scene {
        let text = props.url.clone();
        bsn! {
            template(move |_| {
                validate_url(&props.url)?;
                Ok(LinkUrl(props.url.clone()))
            })
            template(move |_| props.colors.clone().initial())
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
        let url = WidgetryLinkButton::url(world, entity)?;
        if let Err(error) = (world.resource::<BrowserOpener>().0)(url) {
            widgetry_warn!(?entity, url, %error, "请求系统浏览器打开链接失败");
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
    mode: Res<WidgetryThemeMode>,
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
        apply_style(&mode.colors().link_button, item, &texts, &mut commands);
    }
}

fn removed_styles(
    mode: Res<WidgetryThemeMode>,
    mut pressed: RemovedComponents<Pressed>,
    mut disabled: RemovedComponents<InteractionDisabled>,
    mut roots: Query<StyleData, With<WidgetryLinkButton>>,
    texts: Query<Has<Underline>, With<LinkText>>,
    mut commands: Commands,
) {
    for entity in pressed.read().chain(disabled.read()) {
        if let Ok(item) = roots.get_mut(entity) {
            apply_style(&mode.colors().link_button, item, &texts, &mut commands);
        }
    }
}

fn refresh_theme(_event: On<WidgetryThemeChanged>, mut commands: Commands) {
    commands.queue(|world: &mut World| -> Result {
        let mode = *world.resource::<WidgetryThemeMode>();
        let mut state = bevy::ecs::system::SystemState::<(
            Query<StyleData, With<WidgetryLinkButton>>,
            Query<Has<Underline>, With<LinkText>>,
            Commands,
        )>::new(world);
        let (mut roots, texts, mut commands) = state.get_mut(world).map_err(|error| {
            widgetry_error!(%error, "LinkButton Theme 颜色查询失败");
            BevyError::error(error.to_string())
        })?;
        for item in &mut roots {
            apply_style(&mode.colors().link_button, item, &texts, &mut commands);
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
    use bevy_widgetry_test_utils::{LogCapture, press, primary_click, release, scene_app};
    use std::cell::RefCell;

    thread_local! {
        static REQUESTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn fake_browser(url: &str) -> Result<(), String> {
        REQUESTS.with_borrow_mut(|requests| requests.push(url.to_owned()));
        Err("injected system failure".to_owned())
    }

    #[test]
    fn activation_requests_current_url_once_and_absorbs_native_failure_with_warning() {
        let logs = LogCapture::default();
        logs.run(|| {
            let mut app = scene_app();
            app.add_plugins(crate::WidgetryButtonPlugin);
            app.insert_resource(BrowserOpener(fake_browser));
            let parent = app.world_mut().spawn_scene(bsn! { Node Children [@WidgetryLinkButton { @url: {"https://www.baidu.com".to_owned()} }] }).unwrap().id();
            let root = app.world().get::<Children>(parent).unwrap()[0];
            app.update();
            REQUESTS.with_borrow_mut(Vec::clear);
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
        let warnings: Vec<_> = logs
            .records()
            .into_iter()
            .filter(|record| record.level == bevy::log::tracing::Level::WARN)
            .collect();
        assert_eq!(warnings.len(), 2);
        assert!(warnings.iter().all(|record| {
            record
                .fields
                .get("error")
                .is_some_and(|error| error.contains("injected system failure"))
        }));
    }
}
