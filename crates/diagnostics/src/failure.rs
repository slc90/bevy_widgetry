use crate::ErrorOrigin;
use bevy_widgetry_internal_log::{widgetry_debug, widgetry_error};
use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Debug)]
pub struct ErrorFrame {
    operation: String,
    fields: Vec<(String, String)>,
}

impl ErrorFrame {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            fields: Vec::new(),
        }
    }

    pub fn field(mut self, name: impl Into<String>, value: impl fmt::Display) -> Self {
        self.fields.push((name.into(), value.to_string()));
        self
    }

    pub fn operation(&self) -> &str {
        &self.operation
    }
    pub fn fields(&self) -> &[(String, String)] {
        &self.fields
    }
}

pub struct WidgetryFailure<E> {
    kind: E,
    origin: Arc<ErrorOrigin>,
    contexts: Vec<ErrorFrame>,
}

impl<E: Error + 'static> WidgetryFailure<E> {
    #[track_caller]
    pub fn new(kind: E, context: ErrorFrame) -> Self {
        let failure = Self {
            kind,
            origin: Arc::new(ErrorOrigin::capture()),
            contexts: vec![context],
        };
        widgetry_error!(
            error_id = %failure.origin.id(),
            phase = "source",
            error = %Reasons(&failure.kind),
            context = ?failure.contexts[0],
            "操作失败"
        );
        failure
    }

    pub fn kind(&self) -> &E {
        &self.kind
    }
    pub fn origin(&self) -> &Arc<ErrorOrigin> {
        &self.origin
    }
    pub fn contexts(&self) -> &[ErrorFrame] {
        &self.contexts
    }

    pub fn with_context(mut self, context: ErrorFrame) -> Self {
        self.contexts.push(context);
        self
    }

    pub fn map_kind<T: Error + 'static>(self, map: impl FnOnce(E) -> T) -> WidgetryFailure<T> {
        WidgetryFailure {
            kind: map(self.kind),
            origin: self.origin,
            contexts: self.contexts,
        }
    }

    pub fn record_handled(&self, action: &str, outcome: &str) {
        if self.origin.mark_handled() {
            widgetry_debug!(error_id = %self.origin.id(), phase = "handled", action, outcome, "错误处理完成");
        }
    }
}

impl<E: Error + 'static> fmt::Display for WidgetryFailure<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [error_id={}]", Reasons(&self.kind), self.origin.id())?;
        for context in &self.contexts {
            write!(f, " [{context:?}]")?;
        }
        Ok(())
    }
}

impl<E: Error + 'static> fmt::Debug for WidgetryFailure<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl<E: Error + 'static> Error for WidgetryFailure<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.kind)
    }
}

struct Reasons<'a>(&'a (dyn Error + 'static));

impl fmt::Display for Reasons<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)?;
        let mut source = self.0.source();
        while let Some(error) = source {
            write!(f, ": {error}")?;
            source = error.source();
        }
        Ok(())
    }
}
