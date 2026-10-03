use bevy::log::tracing::{
    self, Event, Level, Subscriber,
    field::{Field, Visit},
};
use bevy::log::tracing_subscriber::{Layer, layer::Context, prelude::*};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct LogCapture(Arc<Mutex<Vec<LogRecord>>>);

#[derive(Clone, Debug)]
pub struct LogRecord {
    pub target: String,
    pub level: Level,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub fields: BTreeMap<String, String>,
}

impl LogCapture {
    // with_default 只替换当前 thread 的 subscriber；ECS worker thread 不会继承捕获 scope，因此相关测试须使用 single-threaded schedule。
    pub fn run<R>(&self, action: impl FnOnce() -> R) -> R {
        tracing::subscriber::with_default(
            bevy::log::tracing_subscriber::registry().with(self.clone()),
            action,
        )
    }

    pub fn records(&self) -> Vec<LogRecord> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

impl<S: Subscriber> Layer<S> for LogCapture {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        if metadata.target() != "bevy_widgetry" {
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
        self.0
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
