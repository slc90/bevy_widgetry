use crate::{ErrorFrame, ErrorOrigin, WidgetryFailure};
use bevy_ecs::error::{BevyError, ContextExt, ErrorContext, Severity};
use std::{convert::Infallible, error::Error, fmt, sync::Arc};

pub trait Diagnostic: Error + Send + Sync + 'static {
    fn origin(&self) -> &Arc<ErrorOrigin>;
    fn contexts(&self) -> &[ErrorFrame];
}

impl<E: Error + Send + Sync + 'static> Diagnostic for WidgetryFailure<E> {
    fn origin(&self) -> &Arc<ErrorOrigin> {
        self.origin()
    }
    fn contexts(&self) -> &[ErrorFrame] {
        self.contexts()
    }
}

pub struct WidgetryEcsError {
    first: Arc<dyn Diagnostic>,
    remaining: Vec<Arc<dyn Diagnostic>>,
}

impl WidgetryEcsError {
    pub fn new<E: Error + Send + Sync + 'static>(failure: WidgetryFailure<E>) -> Self {
        Self::from_shared(Arc::new(failure))
    }

    pub fn from_shared(failure: Arc<dyn Diagnostic>) -> Self {
        Self {
            first: failure,
            remaining: Vec::new(),
        }
    }

    pub fn push_shared(&mut self, failure: Arc<dyn Diagnostic>) {
        self.remaining.push(failure);
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &dyn Diagnostic> {
        std::iter::once(self.first.as_ref()).chain(self.remaining.iter().map(Arc::as_ref))
    }

    pub fn with_context(mut self, context: ErrorFrame) -> Self {
        self.first = contextual(self.first, context.clone());
        self.remaining = self
            .remaining
            .into_iter()
            .map(|failure| contextual(failure, context.clone()))
            .collect();
        self
    }

    pub fn into_bevy_error(self) -> BevyError {
        BevyError::error(self)
    }
}

pub fn into_bevy_error<E: Error + Send + Sync + 'static>(failure: WidgetryFailure<E>) -> BevyError {
    WidgetryEcsError::new(failure).into_bevy_error()
}

#[derive(Default)]
pub struct DiagnosticBatch {
    failures: Vec<Arc<dyn Diagnostic>>,
}

impl DiagnosticBatch {
    pub fn push<E: Error + Send + Sync + 'static>(&mut self, failure: WidgetryFailure<E>) {
        self.push_shared(Arc::new(failure));
    }

    pub fn push_shared(&mut self, failure: Arc<dyn Diagnostic>) {
        self.failures.push(failure);
    }

    pub fn into_bevy_error(self) -> Option<BevyError> {
        let mut failures = self.failures.into_iter();
        let first = failures.next()?;
        Some(
            WidgetryEcsError {
                first,
                remaining: failures.collect(),
            }
            .into_bevy_error(),
        )
    }
}

pub enum BoundaryFailure {
    Widgetry(WidgetryEcsError),
    Native(BevyError),
}

impl BoundaryFailure {
    pub fn with_context(self, context: ErrorFrame) -> Self {
        match self {
            Self::Widgetry(error) => Self::Widgetry(error.with_context(context)),
            Self::Native(error) => {
                // Bevy owns this context list and panic payload. Keep the original object intact.
                let result: Result<Infallible, BevyError> = Err(error);
                match result.context(format!("{context:?}")) {
                    Err(error) => Self::Native(error),
                    Ok(never) => match never {},
                }
            }
        }
    }

    pub fn into_bevy_error(self) -> BevyError {
        match self {
            Self::Widgetry(error) => error.into_bevy_error(),
            Self::Native(error) => error,
        }
    }
}

pub fn dispatch_error(
    error: BevyError,
    context: ErrorContext,
    mut report_managed: impl FnMut(&dyn Diagnostic, &BevyError, &ErrorContext),
    report_native: impl FnOnce(BevyError, ErrorContext),
) {
    if error.severity() == Severity::Error
        && let Some(managed) = error.downcast_ref::<WidgetryEcsError>()
    {
        for diagnostic in managed.diagnostics() {
            report_managed(diagnostic, &error, &context);
        }
        return;
    }
    report_native(error, context);
}

impl fmt::Display for WidgetryEcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, failure) in self.diagnostics().enumerate() {
            if index != 0 {
                f.write_str("; ")?;
            }
            write!(f, "{failure}")?;
        }
        Ok(())
    }
}

impl fmt::Debug for WidgetryEcsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for WidgetryEcsError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.first.as_ref())
    }
}

struct ContextualDiagnostic {
    inner: Arc<dyn Diagnostic>,
    contexts: Vec<ErrorFrame>,
}

fn contextual(inner: Arc<dyn Diagnostic>, context: ErrorFrame) -> Arc<dyn Diagnostic> {
    let mut contexts = inner.contexts().to_vec();
    contexts.push(context);
    Arc::new(ContextualDiagnostic { inner, contexts })
}

impl fmt::Display for ContextualDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)?;
        for context in &self.contexts[self.inner.contexts().len()..] {
            write!(f, " [{context:?}]")?;
        }
        Ok(())
    }
}

impl fmt::Debug for ContextualDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for ContextualDiagnostic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.inner.as_ref())
    }
}

impl Diagnostic for ContextualDiagnostic {
    fn origin(&self) -> &Arc<ErrorOrigin> {
        self.inner.origin()
    }
    fn contexts(&self) -> &[ErrorFrame] {
        &self.contexts
    }
}
