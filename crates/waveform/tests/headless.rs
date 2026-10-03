//! Coverage Map：本文件负责 config/source/cursor/ring/reducer 的 ECS contract。
//! renderer.rs 负责 BSN/layout/asset lifecycle。
//! State：未填满/滚动，读取成功/失败，稳定/改变 output density。
//! Stimuli：cursor 连续推进、倒退、整屏跳转、source partial failure、layout output 改变。
//! Guards：成功必须返回全部 channel 的完整有限 range。
//! Transitions：成功读取整体提交，失败保留旧输出。
//! 同 boundary 为 NoOp，density 变化重建表示。
//! Invariants：channel 对齐、失败保留整份 raw/reduced/viewport、固定容量、NoOp 零读取/零 reduction。
//! Couplings：source 事务提交决定显示位置。
//! 增量 reduction 工作量只随新 sample 和边缘 bucket 增长。

// 测试断言需要在 contract 不满足时立即失败。
// 生产代码的 panic lint 会拒绝这些表达式，因此仅在本测试 scope 允许所列 lint。
#![allow(clippy::disallowed_macros, clippy::unwrap_used)]

use bevy::ecs::schedule::SingleThreadedExecutor;
use bevy::prelude::*;
use bevy_widgetry_test_utils::{ErrorCapture, LogCapture, scene_app};
use bevy_widgetry_waveform::*;
use std::ops::Range;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct Source {
    calls: Mutex<Vec<Range<u64>>>,
    failure: AtomicU8,
}

impl WaveformSource for Source {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError> {
        self.calls.lock().unwrap().push(range.clone());
        let failure = self.failure.load(Ordering::Relaxed);
        for (index, channel) in out.channels_mut().iter_mut().enumerate() {
            if failure != 0 && index > 0 {
                break;
            }
            channel.extend(
                range
                    .clone()
                    .map(|frame| frame as f32 + index as f32 * 1000.0),
            );
        }
        if failure == 1 {
            Err(WaveformReadError("device unavailable".into()))
        } else {
            Ok(())
        }
    }
}

fn fixture(channels: usize, capacity: u32) -> (App, Entity, Arc<Source>) {
    let mut app = scene_app();
    app.add_plugins(WaveformPlugin);
    app.set_error_handler(ErrorCapture::handler());
    app.edit_schedule(Update, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    let source = Arc::new(Source::default());
    let root = app
        .world_mut()
        .spawn((
            WaveformRuntime::new(
                WaveformConfig {
                    sample_rate: 1000,
                    visible_duration_ms: capacity,
                    channel_ranges: vec![-1.0..=1.0; channels],
                },
                source.clone(),
            )
            .unwrap(),
            WaveformCursor::default(),
            WaveformOutputLength(4),
        ))
        .id();
    (app, root, source)
}

fn advance(app: &mut App, root: Entity, frames: u64) {
    app.world_mut()
        .get_mut::<WaveformCursor>(root)
        .unwrap()
        .position = duration_from_frames(frames, 1000).unwrap();
    app.update();
}

fn assert_channels(runtime: &WaveformRuntime) {
    for index in 0..runtime.config().channel_ranges.len() {
        let view = runtime.channel(index).unwrap();
        let actual: Vec<_> = view.first.iter().chain(view.second).copied().collect();
        let expected: Vec<_> = runtime
            .buffered_range()
            .map(|frame| frame as f32 + index as f32 * 1000.0)
            .collect();
        assert_eq!(actual, expected);
    }
}

#[test]
fn cursor_drives_half_open_reads_and_fill_then_scroll() {
    let (mut app, root, source) = fixture(3, 16);
    app.update();
    assert!(source.calls.lock().unwrap().is_empty());
    for (end, viewport, kind) in [
        (4, 0..16, WaveformUpdateKind::IncrementalRead),
        (8, 0..16, WaveformUpdateKind::IncrementalRead),
        (8, 0..16, WaveformUpdateKind::NoOp),
        (20, 4..20, WaveformUpdateKind::IncrementalRead),
        (5, 0..16, WaveformUpdateKind::FullRead),
        (60, 44..60, WaveformUpdateKind::FullRead),
    ] {
        advance(&mut app, root, end);
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert_eq!(runtime.viewport_range(), viewport);
        assert_eq!(runtime.buffered_range(), end.saturating_sub(16)..end);
        assert_eq!(runtime.stats().kind, kind);
        assert_channels(runtime);
    }
    assert_eq!(
        *source.calls.lock().unwrap(),
        [0..4, 4..8, 8..20, 0..5, 44..60]
    );
}

#[test]
fn failed_and_partial_reads_keep_the_last_complete_display() {
    let logs = LogCapture::default();
    let errors = ErrorCapture::default();
    logs.run(|| {
        errors.run(|| {
            let (mut app, root, source) = fixture(2, 16);
            advance(&mut app, root, 12);
            let before = app.world().get::<WaveformRuntime>(root).unwrap();
            let reduced = before.reduced_channels().to_vec();
            let revision = before.revision();
            for (failure, end) in [(1, 15), (1, 60), (2, 14)] {
                source.failure.store(failure, Ordering::Relaxed);
                advance(&mut app, root, end);
                let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
                assert_eq!(runtime.buffered_range(), 0..12);
                assert_eq!(runtime.viewport_range(), 0..16);
                assert_eq!(runtime.reduced_channels(), reduced);
                assert_eq!(runtime.revision(), revision);
                assert_channels(runtime);
            }
            source.failure.store(0, Ordering::Relaxed);
            advance(&mut app, root, 22);
            let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
            assert_eq!(runtime.buffered_range(), 6..22);
            assert_channels(runtime);
        })
    });
    let captured = errors.take();
    assert_eq!(captured.len(), 3);
    assert!(
        captured
            .iter()
            .all(|error| error.severity() == bevy::ecs::error::Severity::Error)
    );
    assert!(captured[0].to_string().contains("device unavailable"));
    let records = logs.records();
    assert_eq!(
        records
            .iter()
            .filter(|record| record.level == bevy::log::tracing::Level::ERROR)
            .count(),
        2
    );
    assert!(records.iter().any(|record| {
        record
            .fields
            .values()
            .any(|value| value.contains("恢复正常"))
    }));
}

#[test]
fn sustained_updates_have_bounded_work_and_memory() {
    let (mut app, root, source) = fixture(4, 1000);
    advance(&mut app, root, 1000);
    let raw_bytes = app
        .world()
        .get::<WaveformRuntime>(root)
        .unwrap()
        .working_set_bytes()
        .0;
    let mut warmed_memory = None;
    for end in (1010..=10_000).step_by(10) {
        advance(&mut app, root, end);
        let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
        assert_channels(runtime);
        assert_eq!(runtime.working_set_bytes().0, raw_bytes);
        assert!(runtime.stats().reduction.visited_samples <= (10 + 250) * 4);
        assert!(runtime.stats().reduction.cached_buckets <= 5 * 4);
        if end == 2000 {
            warmed_memory = Some(runtime.working_set_bytes());
        }
        if end > 2000 {
            assert_eq!(Some(runtime.working_set_bytes()), warmed_memory);
        }
    }
    let before = app.world().get::<WaveformRuntime>(root).unwrap().revision();
    let calls = source.calls.lock().unwrap().len();
    advance(&mut app, root, 10_000);
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert_eq!(runtime.revision(), before);
    assert_eq!(runtime.stats().reduction.visited_samples, 0);
    app.world_mut()
        .get_mut::<WaveformOutputLength>(root)
        .unwrap()
        .0 = 2000;
    app.update();
    let runtime = app.world().get::<WaveformRuntime>(root).unwrap();
    assert!(
        matches!(&runtime.reduced_channels()[0], ReducedChannel::Polyline(points) if points.len() == 1000)
    );
    assert_eq!(source.calls.lock().unwrap().len(), calls);
}
