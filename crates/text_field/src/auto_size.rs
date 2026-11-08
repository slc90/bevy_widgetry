use crate::WidgetryTextFieldHeadless;
use crate::text_field::{TextFieldBase, TextFieldEditor};
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

#[derive(SceneComponent, Default, Clone)]
#[scene(WidgetryTextFieldAutoSizeProps)]
pub struct WidgetryTextFieldAutoSize;

#[derive(Default)]
pub struct WidgetryTextFieldAutoSizeProps {
    pub rows: AutoSize,
}

impl WidgetryTextFieldAutoSize {
    fn scene(props: WidgetryTextFieldAutoSizeProps) -> impl Scene {
        configuration_scene(true, props.rows)
    }
}

pub(crate) fn configuration_scene(multiline: bool, rows: AutoSize) -> impl Scene {
    bsn! {
        WidgetryTextFieldAutoSize
        template(move |context| Configuration::new(multiline, rows).map_err(|error| error.with_context(ErrorFrame::new("text_field.initialize_auto_size").field("root",context.entity.id())).into_bevy_error()))
    }
}
#[derive(PartialEq, Eq)]
enum ResizeKey {
    Height,
    MissingEditor(Entity),
    QueryMismatch(Entity),
    Aliased(Entity),
    Owner(Entity, Entity),
    Missing(Entity, &'static str),
    OutsideRoot(Entity),
    SingleLine(Entity),
    Resource(&'static str),
}
#[derive(Component)]
pub(crate) struct Configuration {
    rows: AutoSize,
    failure: FailureState<ResizeKey>,
}
impl Configuration {
    fn new(multiline: bool, config: AutoSize) -> Result<Self, TextFieldFailure> {
        if !multiline || config.min_rows == 0 || config.min_rows > config.max_rows {
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
    mut roots: Query<
        (
            Entity,
            &mut Configuration,
            Option<&WidgetryTextFieldHeadless>,
            Option<&Node>,
            Has<TextFieldBase>,
        ),
        With<WidgetryTextFieldAutoSize>,
    >,
    mut editors: Query<(
        &mut EditableText,
        Option<&TextFieldEditor>,
        Has<bevy::ui_widgets::TextInput>,
    )>,
    parents: Query<&ChildOf>,
    mut fonts: Option<ResMut<FontCx>>,
    mut layouts: Option<ResMut<LayoutCx>>,
) -> Result {
    let mut failures = DiagnosticBatch::default();
    for (root, mut state, binding, node, default_style) in &mut roots {
        let config = state.rows;
        let Some(binding) = binding else {
            state.report(
                ResizeKey::Missing(root, "WidgetryTextFieldHeadless"),
                || {
                    crate::error::missing(
                        root,
                        root,
                        "WidgetryTextFieldHeadless",
                        "text_field.resize_auto_size",
                    )
                },
                &mut failures,
            );
            continue;
        };
        let editor = binding.editor;
        let frame = || {
            ErrorFrame::new("text_field.resize_auto_size")
                .field("root", root)
                .field("editor", editor)
        };
        let Some(node) = node else {
            state.report(
                ResizeKey::Missing(root, "Node"),
                || crate::error::missing(root, root, "Node", "text_field.resize_auto_size"),
                &mut failures,
            );
            continue;
        };
        if default_style && node.height != Val::Auto {
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
        let (mut edit, owner, text_input) = match editors.get_mut(editor) {
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
                            editor,
                            "EditableText",
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
        let Some(owner) = owner else {
            state.report(
                ResizeKey::Missing(editor, "WidgetryTextFieldEditor"),
                || {
                    crate::error::missing(
                        root,
                        editor,
                        "WidgetryTextFieldEditor",
                        "text_field.resize_auto_size",
                    )
                },
                &mut failures,
            );
            continue;
        };
        if owner.root != root {
            state.report(
                ResizeKey::Owner(editor, owner.root),
                || {
                    failure(
                        TextFieldError::InvalidEditorOwner {
                            root,
                            editor,
                            actual: owner.root,
                        },
                        frame(),
                    )
                },
                &mut failures,
            );
            continue;
        }
        if !parents
            .iter_ancestors(editor)
            .any(|ancestor| ancestor == root)
        {
            state.report(
                ResizeKey::OutsideRoot(editor),
                || failure(TextFieldError::EditorOutsideRoot { root, editor }, frame()),
                &mut failures,
            );
            continue;
        }
        if !text_input {
            state.report(
                ResizeKey::Missing(editor, "TextInput"),
                || crate::error::missing(root, editor, "TextInput", "text_field.resize_auto_size"),
                &mut failures,
            );
            continue;
        }
        if !edit.allow_newlines {
            state.report(
                ResizeKey::SingleLine(editor),
                || {
                    failure(
                        TextFieldError::InvalidAutoSize {
                            multiline: false,
                            config,
                        },
                        frame(),
                    )
                },
                &mut failures,
            );
            continue;
        }
        let Some(fonts) = fonts.as_mut() else {
            state.report(
                ResizeKey::Resource("FontCx"),
                || {
                    failure(
                        TextFieldError::MissingResource {
                            root,
                            resource: "FontCx",
                        },
                        frame(),
                    )
                },
                &mut failures,
            );
            continue;
        };
        let Some(layouts) = layouts.as_mut() else {
            state.report(
                ResizeKey::Resource("LayoutCx"),
                || {
                    failure(
                        TextFieldError::MissingResource {
                            root,
                            resource: "LayoutCx",
                        },
                        frame(),
                    )
                },
                &mut failures,
            );
            continue;
        };
        state.failure.recover(frame);
        let lines = edit
            .editor_mut()
            .layout(fonts, &mut layouts.0)
            .lines()
            .count();
        let rows = lines.clamp(config.min_rows, config.max_rows) as f32;
        if edit.visible_lines != Some(rows) {
            edit.visible_lines = Some(rows);
        }
    }
    failures.into_bevy_error().map_or(Ok(()), Err)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use bevy_widgetry_test_utils::{ErrorCapture, LogCapture};
    use std::error::Error;

    #[test]
    fn missing_layout_resources_keep_domain_errors_and_recover_without_replacing_editors() {
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            bevy::scene::ScenePlugin,
            bevy::input_focus::InputFocusPlugin,
        ));
        app.add_plugins(crate::WidgetryTextFieldAutoSizePlugin);
        app.set_error_handler(ErrorCapture::handler());
        app.edit_schedule(PostUpdate, |schedule| {
            schedule.set_executor(bevy::ecs::schedule::SingleThreadedExecutor::default());
        });
        let root = app
            .world_mut()
            .spawn_scene(bsn! {
                #Root @WidgetryTextFieldHeadless {editor:#Editor} @WidgetryTextFieldAutoSize
                Children [#Editor @crate::WidgetryTextFieldEditor {root:#Root,@multiline:true}]
            })
            .unwrap()
            .id();
        let editor = WidgetryTextFieldHeadless::editor_entity(app.world(), root).unwrap();
        let errors = ErrorCapture::default();
        let logs = LogCapture::default();
        let fonts = app.world_mut().remove_resource::<FontCx>().unwrap();
        logs.run(|| errors.run(|| app.update()));
        let records = errors.take_records();
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].error.severity(),
            bevy::ecs::error::Severity::Error
        );
        let diagnostic = records[0].managed().unwrap().diagnostics().next().unwrap();
        let mut cause: &dyn Error = diagnostic;
        let original = loop {
            if let Some(original) = cause.downcast_ref::<TextFieldFailure>() {
                break original;
            }
            cause = cause.source().unwrap();
        };
        assert!(
            matches!(original.kind(),TextFieldError::MissingResource {root:actual,resource:"FontCx"} if *actual==root)
        );
        assert!(Arc::ptr_eq(original.origin(), diagnostic.origin()));
        logs.run(|| errors.run(|| app.update()));
        assert!(errors.take_records().is_empty());
        app.world_mut().insert_resource(fonts);
        let layouts = app.world_mut().remove_resource::<LayoutCx>().unwrap();
        logs.run(|| errors.run(|| app.update()));
        assert_eq!(errors.take_records().len(), 1);
        app.world_mut().insert_resource(layouts);
        logs.run(|| errors.run(|| app.update()));
        assert!(errors.take_records().is_empty());
        assert_eq!(
            WidgetryTextFieldHeadless::editor_entity(app.world(), root).unwrap(),
            editor
        );
        assert_eq!(
            WidgetryTextFieldHeadless::editor(app.world(), root)
                .unwrap()
                .visible_lines,
            Some(2.0)
        );
    }
}
