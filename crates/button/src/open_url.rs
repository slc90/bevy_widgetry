use crate::open_url_error::{OpenUrlRejection, failure};
use crate::{OpenUrlError, OpenUrlFailure, WidgetryButtonHeadless};
use bevy::prelude::*;
use bevy::ui::InteractionDisabled;
use bevy::ui_widgets::Activate;

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryOpenUrlProps)]
#[require(WidgetryButtonHeadless)]
pub struct WidgetryOpenUrl;

#[derive(Default, Clone, Debug)]
pub struct WidgetryOpenUrlProps {
    pub url: String,
}

#[derive(Component, Clone, Reflect)]
#[component(immutable)]
#[reflect(Component)]
pub struct WidgetryOpenUrlState(String);

impl WidgetryOpenUrlState {
    pub fn url(&self) -> &str {
        &self.0
    }
}

#[derive(Resource)]
struct BrowserOpener(fn(Entity, &str) -> Result<(), OpenUrlFailure>);

impl Default for BrowserOpener {
    fn default() -> Self {
        Self(open_browser)
    }
}

fn open_browser(root: Entity, url: &str) -> Result<(), OpenUrlFailure> {
    winsafe::ShellExecuteEx(&winsafe::SHELLEXECUTEINFO {
        mask: winsafe::co::SEE_MASK::FLAG_NO_UI | winsafe::co::SEE_MASK::NOASYNC,
        verb: Some("open"),
        file: url,
        show: winsafe::co::SW::SHOWNORMAL,
        ..default()
    })
    .map_err(|source| {
        failure(
            OpenUrlError::OpenBrowser {
                root,
                url: url.to_owned(),
                source,
            },
            "open_url.open_browser",
        )
    })
}

fn validate_url(root: Entity, raw: &str) -> Result<(), OpenUrlFailure> {
    let invalid = |reason| {
        failure(
            OpenUrlError::InvalidUrl {
                root,
                url: raw.to_owned(),
                reason,
            },
            "open_url.validate_url",
        )
    };
    // The URL parser repairs missing slashes and strips controls, whereas Shell receives the original string.
    // Reject these inputs before parsing so validation and the native request agree.
    if raw.chars().any(|ch| ch.is_control() || ch.is_whitespace()) || raw.contains('\\') {
        return Err(invalid(OpenUrlRejection::UnsafeCharacters));
    }
    let Some((scheme, authority)) = raw.split_once("://") else {
        return Err(invalid(OpenUrlRejection::Incomplete));
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")) {
        return Err(invalid(OpenUrlRejection::UnsupportedScheme));
    }
    if authority.is_empty() || authority.starts_with(['/', '?', '#']) {
        return Err(invalid(OpenUrlRejection::MissingHost));
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
        .map_err(|source| invalid(OpenUrlRejection::Parse(source)))?;
    if let Some(violation) = violation.get() {
        return Err(invalid(OpenUrlRejection::Syntax(violation)));
    }
    if !parsed.has_host() {
        return Err(invalid(OpenUrlRejection::MissingHost));
    }
    Ok(())
}

fn missing(root: Entity, part: &'static str) -> OpenUrlFailure {
    failure(OpenUrlError::MissingPart { root, part }, "open_url.target")
}

impl WidgetryOpenUrl {
    pub fn url(world: &World, entity: Entity) -> Result<&str, OpenUrlFailure> {
        if world.get::<Self>(entity).is_none() {
            return Err(failure(
                OpenUrlError::InvalidTarget { root: entity },
                "open_url.target",
            ));
        }
        if world.get::<WidgetryButtonHeadless>(entity).is_none() {
            return Err(missing(entity, "WidgetryButtonHeadless"));
        }
        if world.get::<Node>(entity).is_none() {
            return Err(missing(entity, "Node"));
        }
        world
            .get::<WidgetryOpenUrlState>(entity)
            .map(|state| state.0.as_str())
            .ok_or_else(|| missing(entity, "WidgetryOpenUrlState"))
    }

    pub fn set_url_in_world(
        world: &mut World,
        entity: Entity,
        url: impl Into<String>,
    ) -> Result<bool, OpenUrlFailure> {
        let url = url.into();
        if Self::url(world, entity)? == url {
            return Ok(false);
        }
        validate_url(entity, &url)?;
        world.entity_mut(entity).insert(WidgetryOpenUrlState(url));
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

    fn scene(props: WidgetryOpenUrlProps) -> impl Scene {
        bsn! {
            template(move |context| {
                validate_url(context.entity.id(), &props.url).map_err(|error| error.into_bevy_error())?;
                Ok(WidgetryOpenUrlState(props.url.clone()))
            })
            on(activate)
        }
    }
}

fn activate(event: On<Activate>, mut commands: Commands) {
    let entity = event.entity;
    commands.queue(move |world: &mut World| -> Result {
        // Read eligibility and the URL at execution, after preceding queued updates or cancellation.
        if world.get::<WidgetryOpenUrl>(entity).is_none()
            || world.get::<InteractionDisabled>(entity).is_some()
        {
            return Ok(());
        }
        let url = WidgetryOpenUrl::url(world, entity).map_err(|error| error.into_bevy_error())?;
        let opener = world.get_resource::<BrowserOpener>().ok_or_else(|| {
            failure(
                OpenUrlError::MissingResource {
                    root: entity,
                    resource: "BrowserOpener",
                },
                "open_url.activate",
            )
            .into_bevy_error()
        })?;
        if let Err(error) = (opener.0)(entity, url) {
            error.record_handled("放弃本次系统浏览器打开请求", "请求已终止，不重试");
        }
        Ok(())
    });
}

pub(crate) fn install(app: &mut App) {
    app.init_resource::<BrowserOpener>()
        .register_type::<WidgetryOpenUrlState>();
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

    fn fake_browser(root: Entity, url: &str) -> Result<(), OpenUrlFailure> {
        REQUESTS.with_borrow_mut(|requests| requests.push(url.to_owned()));
        let error = failure(
            OpenUrlError::OpenBrowser {
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
            let parent = app.world_mut().spawn_scene(bsn! { Node Children [@WidgetryOpenUrl { @url: {"https://www.baidu.com".to_owned()} }] }).unwrap().id();
            let root = app.world().get::<Children>(parent).unwrap()[0];
            app.update();
            REQUESTS.with_borrow_mut(Vec::clear);
            ORIGINS.with_borrow_mut(Vec::clear);
            errors.run(|| {
            WidgetryOpenUrl::set_url_in_world(app.world_mut(), root, "https://www.google.com").unwrap();
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

    fn successful_browser(_: Entity, url: &str) -> Result<(), OpenUrlFailure> {
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
            .spawn_scene(bsn! { @WidgetryOpenUrl { @url: {"https://example.com".to_owned()} } })
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
    fn real_mouse_and_keyboard_open_once_and_execution_reads_queued_url_updates() {
        use bevy::input_focus::InputFocus;
        use bevy::picking::pointer::{Location, PointerAction, PointerButton, PointerId};
        use bevy::window::PrimaryWindow;
        use bevy_widgetry_test_utils::{
            add_keyboard_dispatch, picking_app, press_key, queue_pointer, spawn_picking_camera,
        };
        for custom in [false, true] {
            let mut app = picking_app();
            app.add_plugins(crate::WidgetryButtonPlugin);
            add_keyboard_dispatch(&mut app);
            app.insert_resource(BrowserOpener(successful_browser));
            REQUESTS.with_borrow_mut(Vec::clear);
            let window = app
                .world_mut()
                .spawn((
                    Window {
                        resolution: (400, 400).into(),
                        ..default()
                    },
                    PrimaryWindow,
                ))
                .id();
            let camera = spawn_picking_camera(&mut app, window, UVec2::splat(400), 1.0);
            let parent = app.world_mut().spawn_scene(bsn! {
                Node template(move |_| Ok(UiTargetCamera(camera)))
                Children [
                    @{custom.then_some(bsn! { @WidgetryOpenUrl { @url: {"https://example.com".to_owned()} } })}
                    @{(!custom).then_some(bsn! { @crate::WidgetryLinkButton { @url: {"https://example.com".to_owned()} } })}
                    Node { width: px(180), height: px(50) }
                ]
            }).unwrap().id();
            let root = app.world().get::<Children>(parent).unwrap()[0];
            app.update();
            app.update();
            assert!(app.world().get::<bevy::ui_widgets::Button>(root).is_none());
            REQUESTS.with_borrow(|r| assert!(r.is_empty()));
            let location = Location {
                target: bevy::camera::RenderTarget::Window(bevy::window::WindowRef::Entity(window))
                    .normalize(None)
                    .unwrap(),
                position: Vec2::new(60.0, 25.0),
            };
            let click = |app: &mut App, button| {
                for action in [
                    PointerAction::Move { delta: Vec2::ONE },
                    PointerAction::Press(button),
                    PointerAction::Release(button),
                ] {
                    queue_pointer(app, PointerId::Mouse, location.clone(), action);
                    app.update();
                }
            };
            click(&mut app, PointerButton::Primary);
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(root));
            click(&mut app, PointerButton::Secondary);
            REQUESTS.with_borrow(|r| assert_eq!(r.as_slice(), ["https://example.com"]));
            for key in [KeyCode::Enter, KeyCode::Space] {
                press_key(&mut app, window, key);
            }
            REQUESTS.with_borrow(|r| assert_eq!(r.len(), 3));
            WidgetryOpenUrl::set_url(
                &mut app.world_mut().commands(),
                root,
                "https://example.org/first",
            );
            WidgetryOpenUrl::set_url(
                &mut app.world_mut().commands(),
                root,
                "https://example.org/final",
            );
            app.world_mut().trigger(Activate { entity: root });
            app.world_mut().flush();
            REQUESTS.with_borrow(|r| {
                assert_eq!(r.len(), 4);
                assert_eq!(r.last().unwrap(), "https://example.org/final");
            });
            app.world_mut()
                .entity_mut(parent)
                .insert(InteractionDisabled);
            click(&mut app, PointerButton::Primary);
            press_key(&mut app, window, KeyCode::Enter);
            app.world_mut().trigger(Activate { entity: root });
            app.world_mut().flush();
            REQUESTS.with_borrow(|r| assert_eq!(r.len(), 4));
            app.world_mut()
                .entity_mut(parent)
                .remove::<InteractionDisabled>();
            click(&mut app, PointerButton::Primary);
            REQUESTS.with_borrow(|r| {
                assert_eq!(r.len(), 5);
                assert_eq!(r.last().unwrap(), "https://example.org/final");
            });
        }
    }

    #[test]
    fn missing_runtime_or_state_report_domain_errors_without_opening() {
        let mut app = scene_app();
        app.add_plugins(crate::WidgetryButtonPlugin);
        app.set_error_handler(ErrorCapture::handler());
        let root = app
            .world_mut()
            .spawn_scene(bsn! { @WidgetryOpenUrl { @url: {"https://example.com".to_owned()} } })
            .unwrap()
            .id();
        app.world_mut().remove_resource::<BrowserOpener>();
        let errors = ErrorCapture::default();
        errors.run(|| {
            app.world_mut().trigger(Activate { entity: root });
            app.world_mut().flush();
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostic = records[0]
            .error
            .downcast_ref::<bevy_widgetry_diagnostics::bevy::WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(
            matches!(diagnostic.source().unwrap().downcast_ref::<OpenUrlError>().unwrap(), OpenUrlError::MissingResource { root: target, resource: "BrowserOpener" } if *target==root)
        );
        app.world_mut()
            .entity_mut(root)
            .remove::<WidgetryOpenUrlState>();
        assert!(
            matches!(WidgetryOpenUrl::set_url_in_world(app.world_mut(),root,"https://bevy.org").unwrap_err().kind(), OpenUrlError::MissingPart { root: target, part: "WidgetryOpenUrlState" } if *target==root)
        );
    }
}
