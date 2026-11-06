use bevy_widgetry_theme::WidgetryTheme;

use bevy::app::App;
pub fn switch_theme(app: &mut App, mode: WidgetryTheme) {
    WidgetryTheme::set(&mut app.world_mut().commands(), mode);
    app.world_mut().flush();
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{DiagnosticPhase, ErrorCapture, LogCapture};
    use bevy::{ecs::error::Severity, prelude::*};
    use bevy_widgetry_diagnostics::{
        ErrorFrame,
        bevy::{Diagnostic, WidgetryEcsError},
    };
    use bevy_widgetry_theme::{ThemeError, WidgetryThemeChanged, WidgetryThemePlugin};
    use std::{backtrace::BacktraceStatus, error::Error, sync::Arc};

    fn kind(diagnostic: &dyn Diagnostic) -> &ThemeError {
        let mut source: &dyn Error = diagnostic;
        loop {
            if let Some(kind) = source.downcast_ref::<ThemeError>() {
                return kind;
            }
            source = source.source().unwrap();
        }
    }

    #[test]
    fn missing_theme_is_a_typed_request_and_preserves_origin_through_real_command() {
        let mut app = App::new();
        app.set_error_handler(ErrorCapture::handler());
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| {
            let first = WidgetryTheme::set_in_world(
                app.world_mut(),
                bevy_widgetry_theme::WIDGETRY_DARK_THEME,
            )
            .unwrap_err();
            let second = WidgetryTheme::set_in_world(
                app.world_mut(),
                bevy_widgetry_theme::WIDGETRY_DARK_THEME,
            )
            .unwrap_err();
            assert!(matches!(first.kind(), ThemeError::MissingTheme));
            assert_ne!(first.origin().id(), second.origin().id());
            let origin = Arc::clone(first.origin());
            assert_eq!(origin.backtrace().status(), BacktraceStatus::Captured);
            assert_eq!(first.contexts()[0].operation(), "theme.set_in_world");
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(errors.take_records().is_empty());
            let first = first.with_context(ErrorFrame::new("theme.consumer"));
            app.world_mut()
                .commands()
                .queue(move |_: &mut World| -> Result { Err(first.into_bevy_error()) });
            errors.run(|| app.world_mut().flush());
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].error.severity(), Severity::Error);
            let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
            assert!(Arc::ptr_eq(diagnostic.origin(), &origin));
            assert!(matches!(kind(diagnostic), ThemeError::MissingTheme));
            assert_eq!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
            assert!(!app.world().contains_resource::<WidgetryTheme>());
        });
    }

    #[test]
    fn queued_switch_validates_missing_resource_at_execution_and_uses_error_severity() {
        let mut app = App::new();
        app.add_plugins(WidgetryThemePlugin);
        app.set_error_handler(ErrorCapture::handler());
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| {
            let mut queue = bevy::ecs::world::CommandQueue::default();
            let mut commands = Commands::new(&mut queue, app.world());
            WidgetryTheme::set(&mut commands, bevy_widgetry_theme::WIDGETRY_DARK_THEME);
            assert!(logs.records().is_empty());
            app.world_mut().remove_resource::<WidgetryTheme>();
            errors.run(|| queue.apply(app.world_mut()));
            let records = errors.take_records();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].error.severity(), Severity::Error);
            let managed = records[0].error.downcast_ref::<WidgetryEcsError>().unwrap();
            let diagnostic = managed.diagnostics().next().unwrap();
            assert!(matches!(kind(diagnostic), ThemeError::MissingTheme));
            assert_eq!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                    .len(),
                1
            );
            assert!(
                logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                    .is_empty()
            );
        });
    }

    #[derive(Resource, Default)]
    struct Notifications(Vec<WidgetryTheme>);

    #[test]
    fn queued_invalid_palette_preserves_state_and_reports_one_typed_error() {
        let mut app = App::new();
        app.add_plugins(WidgetryThemePlugin)
            .init_resource::<Notifications>()
            .set_error_handler(ErrorCapture::handler());
        app.add_observer(
            |_: On<WidgetryThemeChanged>,
             theme: Res<WidgetryTheme>,
             mut seen: ResMut<Notifications>| {
                seen.0.push(*theme);
            },
        );
        let before = *app.world().resource::<WidgetryTheme>();
        let mut invalid = before;
        invalid.text.normal.foreground = Color::WHITE;
        invalid.waveform.disabled.background = Color::linear_rgba(f32::INFINITY, 0.0, 0.0, 1.0);
        let logs = LogCapture::default();
        let errors = ErrorCapture::default();
        logs.run(|| {
            let mut queue = bevy::ecs::world::CommandQueue::default();
            WidgetryTheme::set(&mut Commands::new(&mut queue, app.world()), invalid);
            assert!(logs.records().is_empty());
            errors.run(|| queue.apply(app.world_mut()));
        });
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].error.severity(), Severity::Error);
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        assert!(
            matches!(kind(diagnostic), ThemeError::InvalidColor { field, .. } if field == "theme.waveform.disabled.background")
        );
        assert_eq!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Source)
                .len(),
            1
        );
        assert!(
            logs.diagnostic_records(diagnostic.origin().id(), DiagnosticPhase::Handled)
                .is_empty()
        );
        assert_eq!(*app.world().resource::<WidgetryTheme>(), before);
        assert!(app.world().resource::<Notifications>().0.is_empty());
    }

    #[test]
    fn normal_theme_switch_helper_and_same_value_have_only_original_notifications() {
        let mut app = App::new();
        app.add_plugins(WidgetryThemePlugin)
            .init_resource::<Notifications>();
        app.add_observer(
            |_: On<WidgetryThemeChanged>,
             mode: Res<WidgetryTheme>,
             mut seen: ResMut<Notifications>| {
                seen.0.push(*mode);
            },
        );
        let modes = [
            bevy_widgetry_theme::WIDGETRY_DARK_THEME,
            #[cfg(feature = "light")]
            bevy_widgetry_theme::WIDGETRY_LIGHT_THEME,
            #[cfg(feature = "pink_dream")]
            bevy_widgetry_theme::WIDGETRY_PINK_DREAM_THEME,
            #[cfg(feature = "kamuri_violet")]
            bevy_widgetry_theme::WIDGETRY_KAMURI_VIOLET_THEME,
        ];
        let logs = LogCapture::default();
        logs.run(|| {
            for mode in modes {
                switch_theme(&mut app, mode);
                switch_theme(&mut app, mode);
            }
        });
        assert!(logs.records().is_empty());
        assert_eq!(
            app.world().resource::<Notifications>().0,
            modes.into_iter().skip(1).collect::<Vec<_>>()
        );
    }
}
