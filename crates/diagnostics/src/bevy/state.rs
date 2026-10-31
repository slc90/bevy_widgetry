use super::{Diagnostic, WidgetryEcsError};
use crate::ErrorFrame;
use bevy_widgetry_internal_log::widgetry_info;
use std::sync::Arc;

pub struct FailureState<K> {
    active: Option<ActiveFailure<K>>,
}
struct ActiveFailure<K> {
    key: K,
    diagnostics: WidgetryEcsError,
}

impl<K> Default for FailureState<K> {
    fn default() -> Self {
        Self { active: None }
    }
}

impl<K> FailureState<K> {
    pub fn active(&self) -> Option<&dyn Diagnostic> {
        self.diagnostics().next()
    }
    pub fn diagnostics(&self) -> impl Iterator<Item = &dyn Diagnostic> {
        self.active
            .iter()
            .flat_map(|active| active.diagnostics.diagnostics())
    }
    pub fn recover(&mut self, context: impl FnOnce() -> ErrorFrame) -> bool {
        let Some(active) = self.active.take() else {
            return false;
        };
        let context = context();
        for diagnostic in active.diagnostics.diagnostics() {
            widgetry_info!(error_id = %diagnostic.origin().id(), phase = "recovered", context = ?context, "持续异常恢复正常");
        }
        true
    }
}

impl<K: Eq> FailureState<K> {
    pub fn failure(
        &mut self,
        key: K,
        create: impl FnOnce() -> Arc<dyn Diagnostic>,
    ) -> WidgetryEcsError {
        self.failure_batch(key, || WidgetryEcsError::from_shared(create()))
    }
    pub fn failure_batch(
        &mut self,
        key: K,
        create: impl FnOnce() -> WidgetryEcsError,
    ) -> WidgetryEcsError {
        if let Some(active) = &self.active
            && active.key == key
        {
            return active.diagnostics.clone();
        }
        let diagnostics = create();
        self.active = Some(ActiveFailure {
            key,
            diagnostics: diagnostics.clone(),
        });
        diagnostics
    }
}
