#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::prelude::*;
use bevy_widgetry_button::{
    LinkButtonError, LinkButtonFailure, LinkUrlRejection, WidgetryButtonPlugin, WidgetryLinkButton,
    WidgetryLinkButtonColorOverrides,
};
use bevy_widgetry_core::diagnostics::CoreError;
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;
use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture, scene_app};
use std::error::Error;

fn fixture() -> (App, Entity) {
    let mut app = scene_app();
    app.add_plugins(WidgetryButtonPlugin);
    let root = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryLinkButton { @url: {"https://example.com".to_owned()} }
        })
        .unwrap()
        .id();
    (app, root)
}

#[test]
fn invalid_urls_keep_rejection_parser_source_and_original_state() {
    let (mut app, root) = fixture();
    for (raw, expected) in [
        ("https://bad host", "unsafe"),
        ("example.com", "incomplete"),
        ("ftp://example.com", "scheme"),
        ("https://", "host"),
        ("https://example.com:99999", "parse"),
        ("https://example.com/%GG", "syntax"),
    ] {
        let logs = LogCapture::default();
        logs.run(|| {
            let failure: LinkButtonFailure =
                WidgetryLinkButton::set_url_in_world(app.world_mut(), root, raw).unwrap_err();
            let LinkButtonError::InvalidUrl {
                root: target,
                url,
                reason,
            } = failure.kind()
            else {
                panic!("wrong classification")
            };
            assert_eq!(*target, root);
            assert_eq!(url, raw);
            assert!(matches!(
                (expected, reason),
                ("unsafe", LinkUrlRejection::UnsafeCharacters)
                    | ("incomplete", LinkUrlRejection::Incomplete)
                    | ("scheme", LinkUrlRejection::UnsupportedScheme)
                    | ("host", LinkUrlRejection::MissingHost)
                    | ("parse", LinkUrlRejection::Parse(_))
                    | ("syntax", LinkUrlRejection::Syntax(_))
            ));
            if expected == "parse" {
                assert!(reason.source().unwrap().is::<url::ParseError>());
            }
            assert_eq!(
                WidgetryLinkButton::url(app.world(), root).unwrap(),
                "https://example.com"
            );
            let label = app.world().get::<Children>(root).unwrap()[0];
            assert_eq!(
                app.world().get::<Text>(label).unwrap().0,
                "https://example.com"
            );
            assert_eq!(
                logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        });
    }
}

#[test]
fn link_color_core_failure_keeps_origin_and_field_through_ecs_conversion() {
    let (mut app, root) = fixture();
    let logs = LogCapture::default();
    logs.run(|| {
        let colors = WidgetryLinkButtonColorOverrides {
            hovered: Some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0)),
            ..default()
        };
        let failure = WidgetryLinkButtonColorOverrides::set_in_world(app.world_mut(), root, colors)
            .unwrap_err();
        assert!(matches!(
            failure.kind(),
            LinkButtonError::Core {
                source: CoreError::InvalidColor { .. }
            }
        ));
        assert!(failure.kind().source().unwrap().is::<CoreError>());
        assert!(format!("{:?}", failure.contexts()).contains("hovered"));
        assert_eq!(
            *WidgetryLinkButtonColorOverrides::get(app.world(), root).unwrap(),
            WidgetryLinkButtonColorOverrides::default()
        );
        let origin = failure.origin().clone();
        let report = failure.into_bevy_error();
        let diagnostic = report
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(std::sync::Arc::ptr_eq(diagnostic.origin(), &origin));
        assert_eq!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                .is_empty()
        );
    });
}

#[test]
fn actual_url_and_color_commands_preserve_domain_identity_at_the_host() {
    let (mut app, root) = fixture();
    let stale = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(stale);
    let errors = ErrorCapture::default();
    let logs = LogCapture::default();
    app.set_error_handler(ErrorCapture::handler());
    WidgetryLinkButton::set_url(
        &mut app.world_mut().commands(),
        root,
        "https://example.com:99999",
    );
    WidgetryLinkButtonColorOverrides::clear(&mut app.world_mut().commands(), stale);
    logs.run(|| errors.run(|| app.world_mut().flush()));
    let records = errors.take_records();
    assert_eq!(records.len(), 2);
    for record in records {
        assert_eq!(record.error.severity(), bevy::ecs::error::Severity::Error);
        let diagnostic = record
            .error
            .downcast_ref::<WidgetryEcsError>()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap();
        assert!(diagnostic.source().unwrap().is::<LinkButtonError>());
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
    }
}

#[test]
fn initial_url_and_color_failures_keep_the_original_diagnostic_and_clean_the_root() {
    let (mut app, _) = fixture();
    for invalid_color in [false, true] {
        let count = app.world().entities().len();
        let logs = LogCapture::default();
        logs.run(|| {
            let colors = WidgetryLinkButtonColorOverrides {
                disabled: invalid_color.then_some(Color::linear_rgba(f32::NAN, 0.0, 0.0, 1.0)),
                ..default()
            };
            let url = if invalid_color {
                "https://example.com"
            } else {
                "https://example.com:99999"
            };
            let error = bevy_widgetry_core::scene::spawn_scene(
                app.world_mut(),
                bsn! {
                    @WidgetryLinkButton { @url: {url.to_owned()}, @colors: {colors.clone()} }
                },
            )
            .unwrap_err();
            let diagnostic = error.diagnostics().diagnostics().next().unwrap();
            let mut source: &(dyn Error + 'static) = diagnostic;
            while !source.is::<LinkButtonFailure>() {
                source = source.source().unwrap();
            }
            let failure = source.downcast_ref::<LinkButtonFailure>().unwrap();
            assert!(std::sync::Arc::ptr_eq(
                failure.origin(),
                diagnostic.origin()
            ));
            assert_eq!(
                logs.diagnostic_records(failure.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert_eq!(app.world().entities().len(), count);
        });
    }
}
