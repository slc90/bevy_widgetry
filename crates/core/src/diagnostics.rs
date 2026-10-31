use bevy::prelude::BevyError;
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{Diagnostic, WidgetryEcsError},
};
use bevy_widgetry_internal_log::widgetry_info;
use std::sync::Arc;

pub struct FailureState<K> {
    active: Option<ActiveFailure<K>>,
}

struct ActiveFailure<K> {
    key: K,
    diagnostic: Arc<dyn Diagnostic>,
}

impl<K> Default for FailureState<K> {
    fn default() -> Self {
        Self { active: None }
    }
}

impl<K> FailureState<K> {
    pub fn active(&self) -> Option<&dyn Diagnostic> {
        self.active
            .as_ref()
            .map(|active| active.diagnostic.as_ref())
    }

    pub fn recover(&mut self, context: impl FnOnce() -> ErrorFrame) -> bool {
        let Some(active) = self.active.take() else {
            return false;
        };
        widgetry_info!(
            error_id = %active.diagnostic.origin().id(),
            phase = "recovered",
            context = ?context(),
            "持续异常恢复正常"
        );
        true
    }
}

impl<K: Eq> FailureState<K> {
    pub fn failure(
        &mut self,
        key: K,
        create: impl FnOnce() -> Arc<dyn Diagnostic>,
    ) -> WidgetryEcsError {
        if let Some(active) = &self.active
            && active.key == key
        {
            return WidgetryEcsError::from_shared(Arc::clone(&active.diagnostic));
        }
        let diagnostic = create();
        let error = WidgetryEcsError::from_shared(Arc::clone(&diagnostic));
        self.active = Some(ActiveFailure { key, diagnostic });
        error
    }
}

// Existing Owners adopt explicit business keys with their domain migration.
// Keep their current reporting behavior separate from the keyed state contract.
#[doc(hidden)]
#[derive(Default)]
pub struct UnmigratedFailureState(Option<String>);

impl UnmigratedFailureState {
    pub fn observe<T>(
        &mut self,
        result: Result<T, BevyError>,
        failed: impl FnOnce(&BevyError),
        recovered: impl FnOnce(),
    ) -> Result<T, BevyError> {
        match &result {
            Err(error) => {
                // BevyError Display includes a backtrace on each failure.
                // Compare only the first line so backtrace differences do not report the same failure as a new one.
                let display = error.to_string();
                let message = display.lines().next().unwrap_or_default().to_owned();
                if self.0.as_ref() != Some(&message) {
                    // Managed sources have already emitted their ERROR before reaching this Owner.
                    if !error.is::<WidgetryEcsError>() {
                        failed(error);
                    }
                    self.0 = Some(message);
                }
            }
            Ok(_) => {
                if self.0.take().is_some() {
                    recovered();
                }
            }
        }
        result
    }
}
