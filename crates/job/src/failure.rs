use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{Diagnostic, WidgetryEcsError},
};
use std::{convert::Infallible, error::Error, fmt, sync::Arc};

#[derive(Debug)]
pub enum JobError {
    External {
        source: Arc<dyn Error + Send + Sync>,
    },
    PanicObserved {
        message: String,
    },
    ThreadWait {
        source: std::io::Error,
    },
    ThreadWaitAbandoned,
    PoisonedLock {
        protected: &'static str,
    },
    NativeBevy {
        error: bevy::ecs::error::BevyError,
        description: String,
    },
}

impl fmt::Display for JobError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::External { .. } => f.write_str("background job operation failed"),
            Self::PanicObserved { message } => {
                write!(f, "job panic observed after unwind: {message}")
            }
            Self::ThreadWait { .. } => f.write_str("thread exit query failed"),
            Self::ThreadWaitAbandoned => f.write_str("thread wait returned abandoned"),
            Self::PoisonedLock { protected } => write!(f, "job lock poisoned: {protected}"),
            Self::NativeBevy { description, .. } => f.write_str(description),
        }
    }
}

fn native_description(error: &bevy::ecs::error::BevyError) -> String {
    // The caller adds a context so Bevy's Debug always ends its reason block with a
    // single escaped context-list line. Its following bytes are exactly the trace
    // suffix shared with Display, even when the underlying Debug contains fake frames.
    let display = error.to_string();
    let debug = format!("{error:?}");
    if let Some((_, context_and_trace)) = debug.rsplit_once("\ncontext: ")
        && let Some((_, trace)) = context_and_trace.split_once('\n')
        && let Some(reason) = display.strip_suffix(trace)
    {
        return reason.trim_end().to_owned();
    }
    display.trim_end().to_owned()
}

impl Error for JobError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::External { source } => Some(source.as_ref()),
            Self::ThreadWait { source } => Some(source),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct JobFailure(WidgetryEcsError);

impl JobFailure {
    #[track_caller]
    pub fn external(error: impl Error + Send + Sync + 'static, context: ErrorFrame) -> Self {
        Self::external_shared(Arc::new(error), context)
    }

    #[track_caller]
    pub(crate) fn external_shared(
        error: Arc<dyn Error + Send + Sync>,
        context: ErrorFrame,
    ) -> Self {
        WidgetryFailure::new(JobError::External { source: error }, context).into()
    }

    pub fn diagnostics(&self) -> impl Iterator<Item = &dyn Diagnostic> {
        self.0.diagnostics()
    }

    pub fn with_context(self, context: ErrorFrame) -> Self {
        Self(self.0.with_context(context))
    }

    pub fn combine(mut self, other: Self) -> Self {
        for diagnostic in other.0.shared_diagnostics() {
            if !self
                .0
                .diagnostics()
                .any(|current| current.origin().id() == diagnostic.origin().id())
            {
                self.0.push_shared(diagnostic);
            }
        }
        self
    }

    pub fn record_handled(&self, action: &str, outcome: &str) {
        for diagnostic in self.diagnostics() {
            diagnostic.record_handled(action, outcome);
        }
    }

    pub fn into_bevy_error(self) -> bevy::ecs::error::BevyError {
        self.0.into_bevy_error()
    }

    pub fn into_report(self) -> WidgetryEcsError {
        self.0
    }
}

impl<E: Error + Send + Sync + 'static> From<WidgetryFailure<E>> for JobFailure {
    fn from(error: WidgetryFailure<E>) -> Self {
        Self(WidgetryEcsError::new(error))
    }
}

impl From<WidgetryEcsError> for JobFailure {
    fn from(error: WidgetryEcsError) -> Self {
        Self(error)
    }
}

impl From<std::io::Error> for JobFailure {
    #[track_caller]
    fn from(error: std::io::Error) -> Self {
        if let Some(managed) = error
            .get_ref()
            .and_then(|cause| cause.downcast_ref::<JobFailure>())
        {
            return managed
                .clone()
                .with_context(ErrorFrame::new("job.io_wrapper").field("kind", error.kind()));
        }
        if let Some(managed) = error
            .get_ref()
            .and_then(|cause| cause.downcast_ref::<crate::SendFailure>())
        {
            let origin = managed.origin().clone();
            let mut contexts = managed.contexts().to_vec();
            contexts.push(ErrorFrame::new("job.io_wrapper").field("kind", error.kind()));
            return WidgetryEcsError::from_shared(Arc::new(IoSendDiagnostic {
                error,
                origin,
                contexts,
            }))
            .into();
        }
        Self::external(error, ErrorFrame::new("job.external"))
    }
}

#[derive(Debug)]
struct IoSendDiagnostic {
    error: std::io::Error,
    origin: Arc<bevy_widgetry_diagnostics::ErrorOrigin>,
    contexts: Vec<ErrorFrame>,
}
impl fmt::Display for IoSendDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.error, f)
    }
}
impl Error for IoSendDiagnostic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}
impl Diagnostic for IoSendDiagnostic {
    fn origin(&self) -> &Arc<bevy_widgetry_diagnostics::ErrorOrigin> {
        &self.origin
    }
    fn contexts(&self) -> &[ErrorFrame] {
        &self.contexts
    }
}

impl From<bevy::ecs::error::BevyError> for JobFailure {
    #[track_caller]
    fn from(error: bevy::ecs::error::BevyError) -> Self {
        use bevy::ecs::error::ContextExt;
        let error = match Err::<Infallible, _>(error).context("job.external_bevy_error") {
            Err(error) => error,
            Ok(never) => match never {},
        };
        let description = native_description(&error);
        if let Some(managed) = error.downcast_ref::<WidgetryEcsError>() {
            // Bevy keeps ContextExt messages outside the managed report and exposes
            // no context getter. Preserve their displayed prefix as an extra frame,
            // while retaining the original typed diagnostics and shared origins.
            let reasons = managed.to_string();
            let context = description
                .strip_suffix(&reasons)
                .unwrap_or(&description)
                .trim_end();
            return Self(managed.clone().with_context(
                ErrorFrame::new("job.external_bevy_error").field("bevy_context", context),
            ));
        }
        WidgetryFailure::new(
            JobError::NativeBevy { error, description },
            ErrorFrame::new("job.external_bevy_error"),
        )
        .into()
    }
}

impl PartialEq for JobFailure {
    fn eq(&self, other: &Self) -> bool {
        self.diagnostics()
            .map(|error| error.origin().id())
            .eq(other.diagnostics().map(|error| error.origin().id()))
    }
}
impl Eq for JobFailure {}

impl fmt::Display for JobFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}
impl Error for JobFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.0)
    }
}
