use crate::text_field::{Parts, TextFieldEditor};
use crate::{
    TextFieldError, TextFieldFailure,
    error::{failure, query},
};
use bevy::{
    ecs::query::QueryEntityError,
    prelude::*,
    text::{EditableText, FontCx, LayoutCx},
};
use bevy_widgetry_diagnostics::{
    ErrorFrame,
    bevy::{DiagnosticBatch, FailureState},
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AutoSize {
    pub min_rows: usize,
    pub max_rows: usize,
}
impl Default for AutoSize {
    fn default() -> Self {
        Self {
            min_rows: 2,
            max_rows: 6,
        }
    }
}
#[derive(PartialEq, Eq)]
enum ResizeKey {
    Height,
    MissingEditor(Entity),
    QueryMismatch(Entity),
    Aliased(Entity),
    Owner(Entity, Entity),
}
#[derive(Component)]
pub(crate) struct Configuration {
    rows: Option<AutoSize>,
    failure: FailureState<ResizeKey>,
}
impl Configuration {
    pub(crate) fn new(multiline: bool, config: Option<AutoSize>) -> Result<Self, TextFieldFailure> {
        if let Some(config) = config
            && (!multiline || config.min_rows == 0 || config.min_rows > config.max_rows)
        {
            return Err(failure(
                TextFieldError::InvalidAutoSize { multiline, config },
                ErrorFrame::new("text_field.initialize_auto_size"),
            ));
        }
        Ok(Self {
            rows: config,
            failure: FailureState::default(),
        })
    }
    fn report(
        &mut self,
        key: ResizeKey,
        create: impl FnOnce() -> TextFieldFailure,
        failures: &mut DiagnosticBatch,
    ) {
        let previous = self
            .failure
            .active()
            .map(|diagnostic| diagnostic.origin().clone());
        let error = self.failure.failure(key, || Arc::new(create()));
        for diagnostic in error.shared_diagnostics() {
            if !previous
                .as_ref()
                .is_some_and(|origin| Arc::ptr_eq(origin, diagnostic.origin()))
            {
                failures.push_shared(diagnostic);
            }
        }
    }
}

pub(crate) fn resize_editors(
    mut roots: Query<(Entity, &mut Configuration, &Parts, &Node)>,
    mut editors: Query<(&mut EditableText, &TextFieldEditor)>,
    mut fonts: ResMut<FontCx>,
    mut layouts: ResMut<LayoutCx>,
) -> Result {
    let mut failures = DiagnosticBatch::default();
    for (root, mut state, parts, node) in &mut roots {
        let Some(config) = state.rows else { continue };
        let frame = || {
            ErrorFrame::new("text_field.resize_auto_size")
                .field("root", root)
                .field("editor", parts.editor)
                .field("frame", parts.frame)
        };
        if node.height != Val::Auto {
            state.report(
                ResizeKey::Height,
                || {
                    failure(
                        TextFieldError::HeightConflict {
                            root,
                            height: node.height,
                        },
                        frame(),
                    )
                },
                &mut failures,
            );
            continue;
        }
        let (mut edit, owner) = match editors.get_mut(parts.editor) {
            Ok(editor) => editor,
            Err(source) => {
                let key = match &source {
                    QueryEntityError::NotSpawned(error) => ResizeKey::MissingEditor(error.entity()),
                    QueryEntityError::QueryDoesNotMatch(entity, _) => {
                        ResizeKey::QueryMismatch(*entity)
                    }
                    QueryEntityError::AliasedMutability(entity) => ResizeKey::Aliased(*entity),
                };
                state.report(
                    key,
                    || {
                        query(
                            root,
                            parts.editor,
                            "EditableText / TextFieldEditor",
                            source,
                            "text_field.resize_auto_size",
                        )
                        .with_context(frame())
                    },
                    &mut failures,
                );
                continue;
            }
        };
        if owner.root != root {
            state.report(
                ResizeKey::Owner(parts.editor, owner.root),
                || {
                    failure(
                        TextFieldError::InvalidEditorOwner {
                            root,
                            editor: parts.editor,
                            actual: owner.root,
                        },
                        frame(),
                    )
                },
                &mut failures,
            );
            continue;
        }
        state.failure.recover(frame);
        let lines = edit
            .editor_mut()
            .layout(&mut fonts, &mut layouts.0)
            .lines()
            .count();
        let rows = lines.clamp(config.min_rows, config.max_rows) as f32;
        if edit.visible_lines != Some(rows) {
            edit.visible_lines = Some(rows);
        }
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}
