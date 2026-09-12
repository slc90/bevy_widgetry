use bevy::log::tracing::{
    self, Event, Level, Subscriber,
    field::{Field, Visit},
};
use bevy::log::tracing_subscriber::{Layer, layer::Context, prelude::*};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// 捕获 Widgetry 日志的 metadata 和结构化字段，不初始化全局 subscriber。
#[derive(Clone, Default)]
pub struct LogCapture(Arc<Mutex<Vec<LogRecord>>>);

/// 供行为测试检查的单条日志事实。
#[derive(Clone, Debug)]
pub struct LogRecord {
    /// 统一过滤 target。
    pub target: String,
    /// 宏选择的严重程度。
    pub level: Level,
    /// 调用方的源码位置，由 tracing metadata 提供。
    pub file: Option<String>,
    /// 调用方的源码行号。
    pub line: Option<u32>,
    /// 包括 message 在内的原始结构化字段。
    pub fields: BTreeMap<String, String>,
}

impl LogCapture {
    /// 在线程局部 subscriber 下运行；ECS 测试应使用单线程 schedule，避免工作线程逸出捕获范围。
    pub fn run<R>(&self, action: impl FnOnce() -> R) -> R {
        tracing::subscriber::with_default(
            bevy::log::tracing_subscriber::registry().with(self.clone()),
            action,
        )
    }

    /// 返回已捕获事实的快照；中毒时保留已有诊断数据。
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
