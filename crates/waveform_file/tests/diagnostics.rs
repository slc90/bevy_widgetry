#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy_widgetry_test_utils::{DiagnosticPhase, ErrorCapture, LogCapture};
use bevy_widgetry_waveform::*;
use bevy_widgetry_waveform_file::*;
use std::{
    error::Error,
    io::{Seek, SeekFrom, Write},
    path::Path,
    sync::Arc,
};

fn metadata() -> WaveformMetadata {
    WaveformMetadata {
        channel_count: 1,
        channel_labels: vec!["A".into()],
        sample_rate: 1000,
        unit: WaveformUnit::Volts,
        kind: WaveformSessionKind::Replay { total_frames: 1024 },
    }
}

fn build(path: &Path) {
    build_recording(path, metadata(), |_, range, out| {
        out.extend(range.map(|_| Some(1.0)));
        Ok(())
    })
    .unwrap();
}

fn failure(error: &WaveformReadError) -> &RecordingFailure {
    (error.diagnostics().next().unwrap() as &dyn Error)
        .downcast_ref()
        .unwrap()
}

fn has_path(error: &WaveformReadError, path: &Path) -> bool {
    error
        .diagnostics()
        .flat_map(|d| d.contexts())
        .flat_map(|c| c.fields())
        .any(|(key, value)| key == "path" && value == &path.display().to_string())
}

fn has_field(error: &WaveformReadError, key: &str, value: &str) -> bool {
    error
        .diagnostics()
        .flat_map(|d| d.contexts())
        .flat_map(|c| c.fields())
        .any(|(k, v)| k == key && v == value)
}

fn header(path: &Path) -> (Vec<u8>, usize, serde_json::Value) {
    let bytes = std::fs::read(path).unwrap();
    let size =
        u64::from_le_bytes(bytes[bytes.len() - 16..bytes.len() - 8].try_into().unwrap()) as usize;
    let start = bytes.len() - 16 - size;
    let value = serde_json::from_slice(&bytes[start..bytes.len() - 16]).unwrap();
    (bytes, start, value)
}

fn write_at(path: &Path, offset: u64, bytes: &[u8]) {
    let mut file = std::fs::OpenOptions::new().write(true).open(path).unwrap();
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.write_all(bytes).unwrap();
}

fn replace_header(path: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    let (mut bytes, start, mut value) = header(path);
    edit(&mut value);
    let json = serde_json::to_vec(&value).unwrap();
    bytes.truncate(start);
    bytes.extend(&json);
    bytes.extend((json.len() as u64).to_le_bytes());
    bytes.extend(b"WFRIDX02");
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn open_and_query_errors_keep_typed_causes_and_file_context() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recording.widx");
    let missing = Recording::open(&path).err().unwrap();
    assert!(has_path(&missing, &path));
    assert!(
        matches!(failure(&missing).kind(), RecordingError::Io { source, .. }
        if source.kind() == std::io::ErrorKind::NotFound)
    );
    assert!(
        failure(&missing)
            .kind()
            .source()
            .unwrap()
            .is::<std::io::Error>()
    );
    build(&path);
    let mut recording = Recording::open(&path).unwrap();
    let query = recording
        .query(0..1025, 10, WaveformAlgorithm::Lttb)
        .err()
        .unwrap();
    assert!(has_path(&query, &path));
    assert!(
        matches!(failure(&query).kind(), RecordingError::Query { range, output_len: 10, total_frames: 1024 } if range == &(0..1025))
    );
    assert!(has_field(&query, "algorithm", "Lttb"));
    let zero = recording
        .query(0..1, 0, WaveformAlgorithm::MinMax)
        .err()
        .unwrap();
    assert!(matches!(
        failure(&zero).kind(),
        RecordingError::Query { output_len: 0, .. }
    ));
    let channels = recording
        .query_channels(0..1, 1, WaveformAlgorithm::MinMax, 0..2)
        .err()
        .unwrap();
    assert!(
        matches!(failure(&channels).kind(), RecordingError::Channels { range, total: 1 } if range == &(0..2))
    );
    let denied = Recording::open(dir.path()).err().unwrap();
    assert!(has_path(&denied, dir.path()));
    assert!(
        matches!(failure(&denied).kind(), RecordingError::Io { source, .. } if source.kind() == std::io::ErrorKind::PermissionDenied)
    );
}

#[test]
fn producer_failure_keeps_origin_and_actual_chunk_without_publishing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recording.widx");
    let original = WaveformReadError::from(std::io::Error::other("producer failed"));
    let origin = original.diagnostics().next().unwrap().origin().clone();
    let error = build_recording(&path, metadata(), |_, _, _| Err(original.clone())).unwrap_err();
    assert!(Arc::ptr_eq(
        error.diagnostics().next().unwrap().origin(),
        &origin
    ));
    assert!(has_path(&error, &path));
    let fields: Vec<_> = error
        .diagnostics()
        .flat_map(|d| d.contexts())
        .flat_map(|c| c.fields())
        .collect();
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "channel" && value == "0")
    );
    assert!(
        fields
            .iter()
            .any(|(key, value)| key == "range" && value == "0..1024")
    );
    assert!(!path.exists());
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn format_and_json_failures_identify_the_actual_file_section() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recording.widx");
    std::fs::write(&path, b"bad").unwrap();
    let truncated = Recording::open(&path).err().unwrap();
    assert!(has_path(&truncated, &path));
    assert!(matches!(
        failure(&truncated).kind(),
        RecordingError::Truncated {
            actual: 3,
            minimum: 24
        }
    ));
    std::fs::write(&path, [0; 24]).unwrap();
    let magic = Recording::open(&path).err().unwrap();
    assert!(
        matches!(failure(&magic).kind(), RecordingError::Magic { offset: 0, actual } if actual == &[0; 8])
    );
    std::fs::remove_file(&path).unwrap();
    build(&path);
    let (bytes, start, _) = header(&path);
    write_at(&path, start as u64, b"?");
    let json = Recording::open(&path).err().unwrap();
    assert!(has_path(&json, &path));
    assert!(
        matches!(failure(&json).kind(), RecordingError::Json { source, offset, length: Some(length) }
        if source.is_syntax() && *offset == start as u64 && *length == (bytes.len() - start - 16) as u64)
    );
    assert!(
        failure(&json)
            .kind()
            .source()
            .unwrap()
            .is::<serde_json::Error>()
    );
    std::fs::write(&path, &bytes).unwrap();
    replace_header(&path, |value| value["unit"] = 9.into());
    let unit = Recording::open(&path).err().unwrap();
    assert!(matches!(
        failure(&unit).kind(),
        RecordingError::Unit { value: 9 }
    ));
    std::fs::write(&path, &bytes).unwrap();
    replace_header(&path, |value| {
        value["levels"][0][0]["table"] = u64::MAX.into()
    });
    let level = Recording::open(&path).err().unwrap();
    assert!(matches!(
        failure(&level).kind(),
        RecordingError::Level {
            channel: 0,
            table: u64::MAX,
            algorithm: WaveformAlgorithm::MinMax,
            ..
        }
    ));
}

#[test]
fn read_and_index_errors_keep_offsets_channels_and_algorithm() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recording.widx");
    build(&path);
    let (bytes, _, header) = header(&path);
    for (algorithm, table) in [
        (
            WaveformAlgorithm::MinMax,
            header["levels"][0][0]["table"].as_u64().unwrap(),
        ),
        (
            WaveformAlgorithm::Lttb,
            header["lttb"][0][0]["table"].as_u64().unwrap(),
        ),
    ] {
        std::fs::write(&path, &bytes).unwrap();
        write_at(&path, table, &u64::MAX.to_le_bytes());
        let width = if algorithm == WaveformAlgorithm::MinMax {
            4
        } else {
            64
        };
        let error = Recording::open(&path)
            .unwrap()
            .query(0..1024, width, algorithm)
            .err()
            .unwrap();
        assert!(matches!(
            failure(&error).kind(),
            RecordingError::IndexEntry {
                offset: u64::MAX,
                ..
            }
        ));
        assert!(has_path(&error, &path));
        assert!(has_field(&error, "channel", "0"));
        assert!(has_field(&error, "algorithm", &format!("{algorithm:?}")));
    }
    std::fs::write(&path, &bytes).unwrap();
    let mut recording = Recording::open(&path).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(8)
        .unwrap();
    let read = recording
        .query(2..4, 2, WaveformAlgorithm::MinMax)
        .err()
        .unwrap();
    assert!(has_path(&read, &path));
    assert!(has_field(&read, "channel", "0"));
    assert!(has_field(&read, "range", "2..4"));
    assert!(
        matches!(failure(&read).kind(), RecordingError::Io { source, offset: Some(18), length: Some(10) }
        if source.kind() == std::io::ErrorKind::UnexpectedEof)
    );
}

#[test]
fn sample_and_producer_contract_errors_preserve_values_and_publication_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recording.widx");
    let length = build_recording(&path, metadata(), |_, _, _| Ok(())).unwrap_err();
    assert!(
        matches!(failure(&length).kind(), RecordingError::ProducerLength { channel: 0, range, actual: 0 } if range == &(0..1024))
    );
    let sample = build_recording(&path, metadata(), |_, range, out| {
        out.extend(range.map(|i| Some(if i == 7 { f32::NAN } else { 1.0 })));
        Ok(())
    })
    .unwrap_err();
    assert!(
        matches!(failure(&sample).kind(), RecordingError::Sample { frame: 7, value, validity: 1 } if value.is_nan())
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    build(&path);
    let original = std::fs::read(&path).unwrap();
    let exists = build_recording(&path, metadata(), |_, range, out| {
        out.extend(range.map(|_| Some(2.0)));
        Ok(())
    })
    .unwrap_err();
    assert!(has_path(&exists, &path));
    assert!(
        matches!(failure(&exists).kind(), RecordingError::Io { source, .. } if source.kind() == std::io::ErrorKind::AlreadyExists)
    );
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    write_at(&path, 8 + 7 * 5, &f32::NAN.to_le_bytes());
    for algorithm in [WaveformAlgorithm::MinMax, WaveformAlgorithm::Lttb] {
        let error = Recording::open(&path)
            .unwrap()
            .query(0..10, 10, algorithm)
            .err()
            .unwrap();
        assert!(
            matches!(failure(&error).kind(), RecordingError::Sample { frame: 7, value, validity: 1 } if value.is_nan())
        );
        assert!(has_field(&error, "algorithm", &format!("{algorithm:?}")));
    }
}

#[test]
fn file_failure_has_one_source_log_and_survives_the_real_command_boundary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.widx");
    let logs = LogCapture::default();
    let error = logs.run(|| Recording::open(&path).err().unwrap());
    let origin = error.diagnostics().next().unwrap().origin().clone();
    let mut app = bevy::prelude::App::new();
    app.set_error_handler(ErrorCapture::handler());
    let capture = ErrorCapture::default();
    logs.run(|| {
        capture.run(|| {
            app.world_mut().commands().queue(
                move |_: &mut bevy::prelude::World| -> bevy::prelude::Result {
                    Err(error.into_bevy_error())
                },
            );
            app.world_mut().flush();
        })
    });
    let records = capture.take_records();
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].error.severity(),
        bevy::ecs::error::Severity::Error
    );
    assert!(Arc::ptr_eq(
        records[0]
            .managed()
            .unwrap()
            .diagnostics()
            .next()
            .unwrap()
            .origin(),
        &origin
    ));
    assert_eq!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Source)
            .len(),
        1
    );
    assert!(
        logs.diagnostic_records(origin.id(), DiagnosticPhase::Handled)
            .is_empty()
    );
}
