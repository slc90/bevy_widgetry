use bevy::{
    camera::NormalizedRenderTarget,
    picking::pointer::{Location, PointerButton, PointerId},
    prelude::*,
    window::SystemCursorIcon,
};

#[derive(Component, Clone, Debug)]
pub struct CursorLease {
    pub(crate) window: Entity,
    pub(crate) button: PointerButton,
    pub(crate) mode: LeaseMode,
    pub(crate) fresh: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum LeaseMode {
    Dragging(SystemCursorIcon),
    Native,
}

impl CursorLease {
    pub fn dragging(
        pointer: PointerId,
        location: &Location,
        button: PointerButton,
        cursor: SystemCursorIcon,
    ) -> Option<Self> {
        Self::new(pointer, location, button, LeaseMode::Dragging(cursor))
    }

    pub fn native(pointer: PointerId, location: &Location, button: PointerButton) -> Option<Self> {
        Self::new(pointer, location, button, LeaseMode::Native)
    }

    fn new(
        pointer: PointerId,
        location: &Location,
        button: PointerButton,
        mode: LeaseMode,
    ) -> Option<Self> {
        let NormalizedRenderTarget::Window(window) = location.target else {
            return None;
        };
        (pointer == PointerId::Mouse && location.position.is_finite()).then_some(Self {
            window: window.entity(),
            button,
            mode,
            fresh: true,
        })
    }
}
