use crate::{
    MessageBoxError, MessageBoxFailure, MessageBoxFault, WidgetryMessageBox, error::frame,
};
use bevy::prelude::*;
use bevy_widgetry_diagnostics::bevy::{FailureState, WidgetryEcsError};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Phase {
    Owner,
    State,
    Palette,
    Part,
}
#[derive(Resource, Default)]
pub(crate) struct MessageBoxDiagnostics(
    HashMap<(Entity, Entity, Phase), FailureState<MessageBoxFault>>,
);
impl MessageBoxDiagnostics {
    pub(crate) fn discard_part(&mut self, entity: Entity) {
        self.0.retain(|(_, part, _), _| *part != entity);
    }
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[track_caller]
pub(crate) fn failure(
    world: &World,
    root: Entity,
    entity: Entity,
    fault: MessageBoxFault,
    operation: &'static str,
) -> MessageBoxFailure {
    MessageBoxFailure::new(
        MessageBoxError::Structure { fault },
        frame(world, root, entity, operation).field("failure", format_args!("{fault:?}")),
    )
}
pub(crate) fn observe(
    world: &mut World,
    root: Entity,
    entity: Entity,
    phase: Phase,
    result: Result<(), MessageBoxFault>,
    operation: &'static str,
) -> Result<(), WidgetryEcsError> {
    world.resource_scope(|world, mut diagnostics: Mut<MessageBoxDiagnostics>| {
        let key = (root, entity, phase);
        match result {
            Err(fault) => Err(diagnostics.0.entry(key).or_default().failure(fault, || {
                Arc::new(failure(world, root, entity, fault, operation))
            })),
            Ok(()) => {
                if let Some(mut state) = diagnostics.0.remove(&key) {
                    state.recover(|| frame(world, root, entity, operation));
                }
                Ok(())
            }
        }
    })
}
pub(crate) fn discard_part(world: &mut World, entity: Entity) {
    world
        .resource_mut::<MessageBoxDiagnostics>()
        .discard_part(entity);
}
pub(crate) fn discard_scope(world: &mut World, root: Entity, entity: Entity) {
    world
        .resource_mut::<MessageBoxDiagnostics>()
        .0
        .retain(|(owner, part, _), _| *owner != root || *part != entity);
}
pub(crate) fn root_removed(
    event: On<Remove<WidgetryMessageBox>>,
    mut diagnostics: ResMut<MessageBoxDiagnostics>,
) {
    diagnostics
        .0
        .retain(|(root, _, _), _| *root != event.entity);
}
pub(crate) fn combine(
    errors: impl IntoIterator<Item = WidgetryEcsError>,
) -> Result<(), WidgetryEcsError> {
    let mut errors = errors.into_iter();
    let Some(mut first) = errors.next() else {
        return Ok(());
    };
    for error in errors {
        for diagnostic in error.shared_diagnostics() {
            first.push_shared(diagnostic);
        }
    }
    Err(first)
}
