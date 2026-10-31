use bevy::prelude::BevyError;
use bevy_widgetry_diagnostics::bevy::WidgetryEcsError;

#[derive(Default)]
pub struct FailureState(Option<String>);

impl FailureState {
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
