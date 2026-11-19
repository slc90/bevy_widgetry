use crate::{
    WindowFailure, WindowFault, maximized::native_maximized, runtime_error,
    window_root::validate_binding,
};
use bevy::{prelude::*, window::WindowCloseRequested};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetryWindowAction {
    Minimize,
    Maximize,
    RestoreFromMaximized,
    RequestClose,
}

impl WidgetryWindowAction {
    /// Returns whether the request was submitted. The native window processes it separately.
    pub fn apply(self, world: &mut World, root: Entity) -> Result<bool, WindowFailure> {
        let native = validate_binding(world, root, false)
            .map_err(|fault| runtime_error::failure(world, root, fault, self.operation()))?
            .0;
        self.apply_bound(world, root, native)
    }

    pub fn queue(self, commands: &mut Commands, root: Entity) {
        commands.queue(move |world: &mut World| -> Result {
            self.apply(world, root)
                .map_err(WindowFailure::into_bevy_error)?;
            Ok(())
        });
    }

    fn operation(self) -> &'static str {
        match self {
            Self::Minimize => "window.minimize",
            Self::Maximize => "window.maximize",
            Self::RestoreFromMaximized => "window.restore_from_maximized",
            Self::RequestClose => "window.request_close",
        }
    }

    fn apply_bound(
        self,
        world: &mut World,
        root: Entity,
        native: Entity,
    ) -> Result<bool, WindowFailure> {
        let Some(mut window) = world.get_mut::<Window>(native) else {
            let fault = WindowFault::MissingPart {
                entity: native,
                component: "Window",
            };
            return Err(runtime_error::failure(world, root, fault, self.operation()));
        };
        match self {
            Self::Minimize if window.enabled_buttons.minimize => window.set_minimized(true),
            Self::Maximize if window.enabled_buttons.maximize => window.set_maximized(true),
            Self::RestoreFromMaximized if window.enabled_buttons.maximize => {
                window.set_maximized(false)
            }
            Self::RequestClose if window.enabled_buttons.close => {
                let Some(mut requests) = world.get_resource_mut::<Messages<WindowCloseRequested>>()
                else {
                    return Err(runtime_error::failure(
                        world,
                        root,
                        WindowFault::MissingCloseRequests,
                        self.operation(),
                    ));
                };
                requests.write(WindowCloseRequested { window: native });
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}

pub(crate) fn queue_toggle_maximize(commands: &mut Commands, root: Entity) {
    commands.queue(move |world: &mut World| -> Result {
        let native = validate_binding(world, root, false)
            .map_err(|fault| {
                runtime_error::failure(world, root, fault, "window.toggle_maximize")
                    .into_bevy_error()
            })?
            .0;
        if let Some(maximized) = native_maximized(native) {
            let action = if maximized {
                WidgetryWindowAction::RestoreFromMaximized
            } else {
                WidgetryWindowAction::Maximize
            };
            action
                .apply_bound(world, root, native)
                .map_err(WindowFailure::into_bevy_error)?;
        }
        Ok(())
    });
}
