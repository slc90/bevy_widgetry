use bevy::{
    ecs::error::Severity,
    platform::cell::SyncCell,
    prelude::*,
    scene::{ApplySceneError, SpawnSceneError},
};
use bevy_widgetry_diagnostics::{
    ErrorFrame, ErrorOrigin, WidgetryFailure,
    bevy::{Diagnostic, WidgetryEcsError},
};
use std::{any::Any, error::Error, fmt, sync::Arc};

pub struct SceneFailure {
    original: Arc<SpawnSceneError>,
    diagnostics: WidgetryEcsError,
    severity: Severity,
    panic_payload: SyncCell<Option<Box<dyn Any + Send>>>,
}

impl SceneFailure {
    #[track_caller]
    pub fn from_native(mut error: SpawnSceneError, root: Entity, operation: &'static str) -> Self {
        let mut frames = vec![ErrorFrame::new(operation).field("root", root)];
        scene_frames(&error, &mut frames);
        let (severity, panic_payload) = native_strategy(&mut error);
        let managed =
            template_error(&error).and_then(|error| error.downcast_ref::<WidgetryEcsError>());
        let existing: Vec<_> = managed
            .into_iter()
            .flat_map(|error| error.diagnostics())
            .enumerate()
            .map(|(index, diagnostic)| {
                let mut contexts = diagnostic.contexts().to_vec();
                contexts.extend(frames.iter().cloned());
                (index, Arc::clone(diagnostic.origin()), contexts)
            })
            .collect();
        let original = Arc::new(error);
        let mut entries = existing.into_iter();
        let diagnostics = if let Some((index, origin, contexts)) = entries.next() {
            let first = Arc::new(SceneDiagnostic {
                original: Arc::clone(&original),
                index,
                origin,
                contexts,
            });
            let mut diagnostics = WidgetryEcsError::from_shared(first);
            for (index, origin, contexts) in entries {
                diagnostics.push_shared(Arc::new(SceneDiagnostic {
                    original: Arc::clone(&original),
                    index,
                    origin,
                    contexts,
                }));
            }
            diagnostics
        } else {
            let kind = SceneError {
                original: Arc::clone(&original),
                summary: without_native_backtrace(&original.to_string()),
            };
            let mut failure = WidgetryFailure::new(kind, frames.remove(0));
            for frame in frames {
                failure = failure.with_context(frame);
            }
            WidgetryEcsError::new(failure)
        };
        Self {
            original,
            diagnostics,
            severity,
            panic_payload: SyncCell::new(panic_payload),
        }
    }

    pub fn original(&self) -> &SpawnSceneError {
        &self.original
    }
    pub fn diagnostics(&self) -> &WidgetryEcsError {
        &self.diagnostics
    }

    pub fn with_context(mut self, context: ErrorFrame) -> Self {
        self.diagnostics = self.diagnostics.with_context(context);
        self
    }

    pub fn into_bevy_error(mut self) -> BevyError {
        let mut error = self
            .diagnostics
            .into_bevy_error()
            .with_severity(self.severity);
        if let Some(payload) = self.panic_payload.get().take() {
            error = error.with_payload(payload);
        }
        error
    }
}

impl fmt::Display for SceneFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.diagnostics, f)
    }
}
impl fmt::Debug for SceneFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

pub struct SceneError {
    original: Arc<SpawnSceneError>,
    summary: String,
}

impl SceneError {
    pub fn original(&self) -> &SpawnSceneError {
        &self.original
    }
    pub fn native_template_error(&self) -> Option<&BevyError> {
        template_error(&self.original)
    }
}
impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.summary)
    }
}
impl fmt::Debug for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for SceneError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        // BevyError is not a standard Error and its Display may append a private stack.
        // The native getter preserves that object; other Scene causes keep their standard chain.
        template_error(&self.original)
            .is_none()
            .then_some(self.original.as_ref())
    }
}

struct SceneDiagnostic {
    original: Arc<SpawnSceneError>,
    index: usize,
    origin: Arc<ErrorOrigin>,
    contexts: Vec<ErrorFrame>,
}

impl fmt::Display for SceneDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(source) = self.source() {
            write!(f, "{source}")?;
        }
        write!(f, " [{:?}]", self.contexts)
    }
}
impl fmt::Debug for SceneDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for SceneDiagnostic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        template_error(&self.original)?
            .downcast_ref::<WidgetryEcsError>()?
            .diagnostics()
            .nth(self.index)
            .map(|diagnostic| diagnostic as &dyn Error)
    }
}
impl Diagnostic for SceneDiagnostic {
    fn origin(&self) -> &Arc<ErrorOrigin> {
        &self.origin
    }
    fn contexts(&self) -> &[ErrorFrame] {
        &self.contexts
    }
}

fn template_error(error: &SpawnSceneError) -> Option<&BevyError> {
    let SpawnSceneError::ApplySceneError(error) = error else {
        return None;
    };
    let mut error = error;
    loop {
        match error {
            ApplySceneError::TemplateBuildError(error) => return Some(error),
            ApplySceneError::CachedSceneApplyError { error: inner, .. }
            | ApplySceneError::RelatedSceneError { error: inner, .. } => error = inner,
            _ => return None,
        }
    }
}

pub(super) fn has_managed_diagnostics(error: &SpawnSceneError) -> bool {
    template_error(error).is_some_and(|error| error.is::<WidgetryEcsError>())
}

fn template_error_mut(error: &mut SpawnSceneError) -> Option<&mut BevyError> {
    let SpawnSceneError::ApplySceneError(error) = error else {
        return None;
    };
    let mut error = error;
    loop {
        match error {
            ApplySceneError::TemplateBuildError(error) => return Some(error),
            ApplySceneError::CachedSceneApplyError { error: inner, .. }
            | ApplySceneError::RelatedSceneError { error: inner, .. } => error = inner,
            _ => return None,
        }
    }
}

fn native_strategy(error: &mut SpawnSceneError) -> (Severity, Option<Box<dyn Any + Send>>) {
    let native = template_error_mut(error);
    let severity = native
        .as_ref()
        .map_or(Severity::Error, |error| error.severity());
    (severity, native.and_then(BevyError::take_payload))
}

pub(super) fn into_native_bevy_error(mut error: SpawnSceneError) -> BevyError {
    let (severity, payload) = native_strategy(&mut error);
    let mut error = BevyError::new(severity, error);
    if let Some(payload) = payload {
        error = error.with_payload(payload);
    }
    error
}

fn scene_frames(error: &SpawnSceneError, frames: &mut Vec<ErrorFrame>) {
    let SpawnSceneError::ApplySceneError(error) = error else {
        return;
    };
    let mut error = error;
    loop {
        match error {
            ApplySceneError::CachedSceneApplyError {
                cached,
                error: inner,
            } => {
                frames.push(
                    ErrorFrame::new("CachedSceneApply").field("cached", format!("{cached:?}")),
                );
                error = inner;
            }
            ApplySceneError::RelatedSceneError {
                relationship_type_name,
                index,
                error: inner,
            } => {
                frames.push(
                    ErrorFrame::new("RelatedScene")
                        .field("relationship", relationship_type_name)
                        .field("index", index),
                );
                error = inner;
            }
            _ => return,
        }
    }
}

fn without_native_backtrace(formatted: &str) -> String {
    // Bevy 0.20 appends its private backtrace after the cause text and exposes no stack-free formatter.
    // Keep the original object separately; this is only the source event's display projection.
    formatted
        .lines()
        .take_while(|line| {
            !line.starts_with("   ")
                || !line
                    .trim_start()
                    .split_once(':')
                    .is_some_and(|(index, frame)| {
                        !index.is_empty()
                            && index.bytes().all(|byte| byte.is_ascii_digit())
                            && (frame.contains("::") || frame.trim_start().starts_with("0x"))
                    })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub struct SceneRejection(Box<dyn Error + Send + Sync>);
impl fmt::Display for SceneRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl fmt::Debug for SceneRejection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}
impl Error for SceneRejection {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

#[track_caller]
pub fn source_error(
    error: impl Into<Box<dyn Error + Send + Sync>>,
    context: ErrorFrame,
) -> BevyError {
    bevy_widgetry_diagnostics::bevy::into_bevy_error(WidgetryFailure::new(
        SceneRejection(error.into()),
        context,
    ))
}

pub fn original_scene_error(diagnostic: &dyn Diagnostic) -> Option<&SpawnSceneError> {
    let mut current: &(dyn Error + 'static) = diagnostic;
    loop {
        if let Some(scene) = current.downcast_ref::<SceneDiagnostic>() {
            return Some(&scene.original);
        }
        if let Some(scene) = current.downcast_ref::<SceneError>() {
            return Some(scene.original());
        }
        current = current.source()?;
    }
}

#[macro_export]
macro_rules! scene_source_error {
    ($cause:expr $(,)?; $($fields:tt)+) => {{
        let context = $crate::scene_source_error!(@frame []; $($fields)+);
        $crate::scene::source_error($cause, context)
    }};
    (@frame [$($built:tt)*]; $name:ident = ?$value:expr, $($rest:tt)+) => {
        $crate::scene_source_error!(@frame [$($built)* .field(stringify!($name), format!("{:?}", &$value))]; $($rest)+)
    };
    (@frame [$($built:tt)*]; $name:ident = %$value:expr, $($rest:tt)+) => {
        $crate::scene_source_error!(@frame [$($built)* .field(stringify!($name), format!("{}", &$value))]; $($rest)+)
    };
    (@frame [$($built:tt)*]; $name:ident = $value:expr, $($rest:tt)+) => {
        $crate::scene_source_error!(@frame [$($built)* .field(stringify!($name), format!("{:?}", &$value))]; $($rest)+)
    };
    (@frame [$($built:tt)*]; ?$name:ident, $($rest:tt)+) => {
        $crate::scene_source_error!(@frame [$($built)* .field(stringify!($name), format!("{:?}", &$name))]; $($rest)+)
    };
    (@frame [$($built:tt)*]; %$name:ident, $($rest:tt)+) => {
        $crate::scene_source_error!(@frame [$($built)* .field(stringify!($name), format!("{}", &$name))]; $($rest)+)
    };
    (@frame [$($built:tt)*]; $name:ident, $($rest:tt)+) => {
        $crate::scene_source_error!(@frame [$($built)* .field(stringify!($name), format!("{:?}", &$name))]; $($rest)+)
    };
    (@frame [$($built:tt)*]; $message:expr $(,)?) => {
        $crate::scene::ErrorFrame::new($message) $($built)*
    };
}
