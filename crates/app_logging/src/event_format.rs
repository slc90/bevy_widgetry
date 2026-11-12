use crate::diagnostic_format::sanitize_text;
use std::fmt;
use tracing::{Event, Subscriber};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    fmt::{
        FmtContext, FormatEvent, FormatFields, FormattedFields, format::Writer, time::FormatTime,
    },
    registry::LookupSpan,
};

pub(crate) struct FileEventFormat<T>(pub T);

pub(crate) struct DiagnosticFields;

impl<'writer> FormatFields<'writer> for DiagnosticFields {
    fn format_fields<R: tracing_subscriber::field::RecordFields>(
        &self,
        mut writer: Writer<'writer>,
        fields: R,
    ) -> fmt::Result {
        let mut text = String::new();
        tracing_subscriber::fmt::format::DefaultFields::new()
            .format_fields(Writer::new(&mut text), fields)?;
        writer.write_str(&sanitize_text(&text))
    }
}

impl<S, N, T> FormatEvent<S, N> for FileEventFormat<T>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
    T: FormatTime,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        self.0.format_time(&mut writer)?;
        write!(writer, " {:>5} ", event.metadata().level())?;
        let thread = std::thread::current();
        if let Some(name) = thread.name() {
            write!(writer, "{name} ")?;
        }
        write!(writer, "{:0>2?} ", thread.id())?;
        if let Some(scope) = ctx.event_scope() {
            for span in scope.from_root() {
                write!(writer, "{}", span.name())?;
                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<FormattedFields<N>>()
                    && !fields.is_empty()
                {
                    write!(writer, "{{{fields}}}")?;
                }
                writer.write_str(":")?;
            }
            writer.write_str(" ")?;
        }
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        let module = metadata
            .module_path()
            .filter(|value| !value.is_empty())
            .or_else(|| (!metadata.target().is_empty()).then_some(metadata.target()))
            .unwrap_or("未知源码");
        writer.write_str(module)?;
        match (metadata.file().and_then(source_filename), metadata.line()) {
            (Some(file), Some(line)) => write!(writer, " ({file}:{line})")?,
            (Some(file), None) => write!(writer, " ({file})")?,
            (None, Some(line)) => write!(writer, " (line {line})")?,
            (None, None) => {}
        }
        writer.write_str(": ")?;
        ctx.format_fields(writer.by_ref(), event)?;
        writeln!(writer)
    }
}

fn source_filename(path: &str) -> Option<&str> {
    let file = path.rsplit(['/', '\\']).next()?;
    (!file.is_empty() && file != "." && file != ".." && !file.contains([':', '\n', '\r']))
        .then_some(file)
}

#[cfg(test)]
#[allow(clippy::disallowed_macros, clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing::{Level, Metadata, callsite, field::FieldSet, metadata::Kind};
    use tracing_subscriber::prelude::*;

    struct SharedBytes(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for SharedBytes {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn filenames_do_not_depend_on_the_build_directory_or_separator() {
        for (input, expected) in [
            (
                r"C:\Users\builder\.cargo\registry\src\index\bevy_app-0.20.0\src\app.rs",
                Some("app.rs"),
            ),
            ("/tmp/install/src/lib.rs", Some("lib.rs")),
            (r"D:\构建 目录/src\main.rs", Some("main.rs")),
            ("plain.rs", Some("plain.rs")),
            ("", None),
            (r"C:\", None),
            ("..", None),
            ("C:", None),
        ] {
            assert_eq!(source_filename(input), expected, "{input}");
        }
    }

    struct FixedTime;
    impl FormatTime for FixedTime {
        fn format_time(&self, writer: &mut Writer<'_>) -> fmt::Result {
            writer.write_str("2026-11-12 13:14:15.123")
        }
    }

    #[test]
    fn real_events_keep_context_and_use_safe_metadata_with_fallbacks() {
        static CALLSITE: callsite::DefaultCallsite = callsite::DefaultCallsite::new(&META);
        static META: Metadata<'static> = Metadata::new(
            "synthetic",
            "bevy_app",
            Level::DEBUG,
            Some(r"C:\Users\builder\.cargo\registry\src\index\bevy_app-0.20.0\src\app.rs"),
            Some(528),
            Some("bevy_app::app"),
            FieldSet::new(&["message"], callsite::Identifier(&CALLSITE)),
            Kind::EVENT,
        );
        static FALLBACK_SITE: callsite::DefaultCallsite = callsite::DefaultCallsite::new(&FALLBACK);
        static FALLBACK: Metadata<'static> = Metadata::new(
            "fallback",
            "external_target",
            Level::WARN,
            None,
            None,
            None,
            FieldSet::new(&[], callsite::Identifier(&FALLBACK_SITE)),
            Kind::EVENT,
        );
        let bytes = Arc::new(Mutex::new(Vec::<u8>::new()));
        let output = bytes.clone();
        let subscriber = tracing_subscriber::registry().with(
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .event_format(FileEventFormat(FixedTime))
                .with_writer(move || SharedBytes(output.clone())),
        );
        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("operation", job_id = 12);
            let _entered = span.enter();
            tracing::info!(target: "widget_gallery", value = 7, "显式 target");
            tracing_log::format_trace(
                &tracing_log::log::Record::builder()
                    .args(format_args!("Log 桥接事件"))
                    .level(tracing_log::log::Level::Debug)
                    .target("bevy_app")
                    .module_path(Some("bevy_app::app"))
                    .file(Some(
                        r"C:\Users\builder\.cargo\registry\src\index\bevy_app-0.20.0\src\app.rs",
                    ))
                    .line(Some(529))
                    .build(),
            )
            .unwrap();
            let message = META.fields().field("message").unwrap();
            Event::dispatch(
                &META,
                &META
                    .fields()
                    .value_set(&[(&message, Some(&"第三方事件" as &dyn tracing::Value))]),
            );
            Event::dispatch(&FALLBACK, &FALLBACK.fields().value_set(&[]));
            for (target, file, line) in [
                ("file_only", Some(r"D:\build\lib.rs"), None),
                ("line_only", None, Some(17)),
                ("", Some(r"D:\"), None),
            ] {
                let metadata = Box::leak(Box::new(Metadata::new(
                    "partial",
                    target,
                    Level::INFO,
                    file,
                    line,
                    None,
                    FieldSet::new(&[], callsite::Identifier(&FALLBACK_SITE)),
                    Kind::EVENT,
                )));
                Event::dispatch(metadata, &metadata.fields().value_set(&[]));
            }
        });
        let output = String::from_utf8(bytes.lock().unwrap().clone()).unwrap();
        assert!(output.contains("bevy_app::app (app.rs:528)"), "{output}");
        assert!(
            output.contains("bevy_app::app (app.rs:529): Log 桥接事件"),
            "{output}"
        );
        assert!(
            output.contains("bevy_widgetry_app_logging::event_format::tests (event_format.rs:")
        );
        assert!(output.contains("external_target"));
        assert!(output.contains("file_only (lib.rs)"));
        assert!(output.contains("line_only (line 17)"));
        assert!(output.contains("未知源码:"));
        assert!(!output.contains(r"D:\"));
        assert!(!output.contains(r"C:\Users\builder"));
        assert!(output.contains("2026-11-12 13:14:15.123"));
        assert!(output.contains("ThreadId("));
        assert!(output.contains(std::thread::current().name().unwrap()));
        assert!(output.contains("INFO"));
        assert!(output.contains("DEBUG"));
        assert!(output.contains("WARN"));
        assert!(output.contains("operation{job_id=12}"));
        assert!(output.contains("value=7"));
        assert!(output.contains("第三方事件"));
    }
}
