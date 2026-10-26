#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::panic)]

use bevy_widgetry_waveform::*;
use bevy_widgetry_waveform_file::*;
use std::time::{Duration, Instant};

fn request(id: u64, reader_id: u64) -> WaveformReadRequest {
    WaveformReadRequest {
        id,
        reader_id,
        session_generation: 1,
        target_generation: id,
        range: id * 100..id * 100 + 1000,
        viewport: id * 100..id * 100 + 1000,
        output_len: 4,
        algorithm: WaveformAlgorithm::MinMax,
    }
}

fn wait(
    source: &DiskWaveformSource,
    request: &WaveformReadRequest,
) -> Result<Vec<ReducedChannel>, WaveformReadError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match source.read_reduced(request)? {
            WaveformReducedReadStatus::Ready {
                request: reply,
                channels,
            } => {
                assert_eq!(&reply, request);
                return Ok(channels);
            }
            WaveformReducedReadStatus::Pending => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
    }
}

#[test]
fn latest_requests_readers_cancellation_and_disk_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("source.widx");
    build_recording(
        &path,
        WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 1000,
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Replay {
                total_frames: 100_000,
            },
        },
        |_, range, out| {
            out.extend(range.map(|frame| Some(frame as f32)));
            Ok(())
        },
    )
    .unwrap();
    bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
    let manager = bevy_widgetry_job::JobManager::default();
    let source = DiskWaveformSource::open(&path, &manager).unwrap();
    for id in 1..400 {
        let _ = source.read_reduced(&request(id, 1)).unwrap();
    }
    let latest = request(400, 1);
    let other = request(20, 2);
    assert!(matches!(
        source.read_reduced(&latest).unwrap(),
        WaveformReducedReadStatus::Pending
    ));
    assert!(matches!(
        source.read_reduced(&other).unwrap(),
        WaveformReducedReadStatus::Pending
    ));
    let channels = wait(&source, &latest).unwrap();
    let stats = source.stats().unwrap();
    assert!(stats.queries > 0);
    let observed = stats.last_read.unwrap();
    assert!(observed.read.bytes_read > 0);
    assert!(observed.elapsed >= observed.read.io_time);
    assert!(observed.read.io_time > Duration::ZERO);
    let ReducedChannel::Envelope(spans) = &channels[0] else {
        panic!("expected envelope")
    };
    assert_eq!(spans.first().unwrap().min, latest.range.start as f32);
    assert_eq!(spans.last().unwrap().max, (latest.range.end - 1) as f32);
    wait(&source, &other).unwrap();
    for id in 401..450 {
        let mut request = request(id, 1);
        request.algorithm = if id % 2 == 0 {
            WaveformAlgorithm::Lttb
        } else {
            WaveformAlgorithm::MinMax
        };
        source.read_reduced(&request).unwrap();
    }
    let mut lttb = request(450, 1);
    lttb.algorithm = WaveformAlgorithm::Lttb;
    let channels = wait(&source, &lttb).unwrap();
    assert!(matches!(&channels[0], ReducedChannel::Polyline(_)));
    let pending = request(10, 1);
    source.read_reduced(&pending).unwrap();
    source.cancel_reads(1);
    assert!(matches!(
        source.read_reduced(&latest).unwrap(),
        WaveformReducedReadStatus::Pending
    ));
    wait(&source, &latest).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(16)
        .unwrap();
    let error = wait(&source, &request(30, 1)).unwrap_err();
    assert!(!error.to_string().is_empty());
}

#[test]
fn reader_creation_respects_global_shutdown() {
    bevy::tasks::IoTaskPool::get_or_init(bevy::tasks::TaskPool::new);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shutdown.widx");
    build_recording(
        &path,
        WaveformMetadata {
            channel_count: 1,
            channel_labels: vec!["A".into()],
            sample_rate: 1000,
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Replay { total_frames: 100 },
        },
        |_, range, out| {
            out.extend(range.map(|_| Some(1.0)));
            Ok(())
        },
    )
    .unwrap();
    let manager = bevy_widgetry_job::JobManager::default();
    manager.begin_shutdown(Duration::ZERO).unwrap();
    assert!(
        matches!(DiskWaveformSource::open(&path, &manager), Err(error) if error.to_string().contains("ShuttingDown"))
    );
}
