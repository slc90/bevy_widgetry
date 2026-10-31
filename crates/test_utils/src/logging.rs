use bevy::log::tracing::{
    self, Event, Level, Subscriber,
    field::{Field, Visit},
};
use bevy::log::tracing_subscriber::{Layer, layer::Context, prelude::*};
use bevy_widgetry_diagnostics::ErrorId;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct LogCapture {
    records: Arc<Mutex<Vec<LogRecord>>>,
    targets: Arc<BTreeSet<String>>,
}

impl Default for LogCapture {
    fn default() -> Self {
        Self::with_targets(["bevy_widgetry"])
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticPhase {
    Source,
    Handled,
    Unhandled,
}

#[derive(Clone, Debug)]
pub struct LogRecord {
    pub target: String,
    pub level: Level,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub fields: BTreeMap<String, String>,
}

impl LogCapture {
    pub fn with_targets(targets: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            records: Arc::default(),
            targets: Arc::new(targets.into_iter().map(Into::into).collect()),
        }
    }

    pub fn dispatcher(&self) -> tracing::Dispatch {
        tracing::Dispatch::new(bevy::log::tracing_subscriber::registry().with(self.clone()))
    }

    // with_default replaces only the current thread's subscriber.
    // ECS workers do not inherit the capture scope, so captured systems must use a single-threaded schedule.
    pub fn run<R>(&self, action: impl FnOnce() -> R) -> R {
        tracing::dispatcher::with_default(&self.dispatcher(), action)
    }

    pub fn records(&self) -> Vec<LogRecord> {
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn records_for_error(&self, id: &ErrorId) -> Vec<LogRecord> {
        let id = id.to_string();
        self.records()
            .into_iter()
            .filter(|record| record.matches_field("error_id", &id))
            .collect()
    }

    pub fn diagnostic_records(&self, id: &ErrorId, phase: DiagnosticPhase) -> Vec<LogRecord> {
        self.records_for_error(id)
            .into_iter()
            .filter(|record| record.phase() == Some(phase))
            .collect()
    }
}

impl LogRecord {
    pub fn matches_field(&self, name: &str, expected: &str) -> bool {
        self.fields
            .get(name)
            .is_some_and(|value| value == expected || value == &format!("{expected:?}"))
    }

    pub fn phase(&self) -> Option<DiagnosticPhase> {
        [
            ("source", DiagnosticPhase::Source),
            ("handled", DiagnosticPhase::Handled),
            ("unhandled", DiagnosticPhase::Unhandled),
        ]
        .into_iter()
        .find_map(|(name, phase)| self.matches_field("phase", name).then_some(phase))
    }
}

impl<S: Subscriber> Layer<S> for LogCapture {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        if !self.targets.contains(metadata.target()) {
            return;
        }
        let mut record = LogRecord {
            target: metadata.target().into(),
            level: *metadata.level(),
            file: metadata.file().map(String::from),
            line: metadata.line(),
            fields: BTreeMap::new(),
        };
        event.record(&mut record);
        self.records
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(record);
    }
}

impl Visit for LogRecord {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields
            .insert(field.name().into(), format!("{value:?}"));
    }
}
