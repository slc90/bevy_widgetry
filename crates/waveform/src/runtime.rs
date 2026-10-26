use crate::source::PlanarRing;
use crate::{
    ChannelView, MinMaxReducer, PlanarBuffer, ReducedChannel, ReductionInput, ReductionStats,
    WaveformConfig, WaveformCursor, WaveformReducer, WaveformSource, sample_boundary,
};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, TaskPool};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use bevy_widgetry_job::{
    JobAccessError, JobId, JobManager, JobManagerPlugin, JobMessage, JobOptions, JobOutcome,
    JobReceiver, SendError, TryRecvError,
};
use std::ops::Range;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

#[derive(Component)]
pub struct WaveformRuntime {
    manager: JobManager,
    config: WaveformConfig,
    source: Option<Arc<dyn WaveformSource>>,
    source_reduced: bool,
    metadata: Option<crate::WaveformMetadata>,
    generation: u64,
    running: bool,
    preview: bool,
    rebuild: bool,
    duration_refresh: bool,
    requested_position: Option<std::time::Duration>,
    reducer: Box<dyn WaveformReducer>,
    algorithm: crate::WaveformAlgorithm,
    algorithm_refresh: bool,
    ring: Arc<PlanarRing>,
    pending_reduction: Option<PendingReduction>,
    reduction_generation: u64,
    staging: PlanarBuffer,
    viewport: Range<u64>,
    reduced: Vec<ReducedChannel>,
    output_len: usize,
    revision: u64,
    stats: WaveformUpdateStats,
    failure: FailureState,
    last_error: Option<String>,
    pending_request: Option<crate::WaveformReadRequest>,
    reader_id: u64,
    target_generation: u64,
}

#[derive(Component, Default)]
pub struct WaveformOutputLength(pub usize);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaveformUpdateKind {
    #[default]
    NoOp,
    IncrementalRead,
    FullRead,
    Pending,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct WaveformUpdateStats {
    pub kind: WaveformUpdateKind,
    pub read_frames: usize,
    pub reduction: ReductionStats,
}

pub struct WaveformPlugin;

struct ReductionResult {
    reducer: Box<dyn WaveformReducer>,
    reduced: Vec<ReducedChannel>,
    stats: ReductionStats,
}

struct PendingReduction {
    job: JobId,
    receiver: JobReceiver<()>,
    handoff: Arc<Mutex<Option<ReductionResult>>>,
    received: bool,
    algorithm: crate::WaveformAlgorithm,
    ring: Arc<PlanarRing>,
    viewport: Range<u64>,
    output_len: usize,
    generation: u64,
    target_generation: u64,
    stats: WaveformUpdateStats,
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WaveformSystems {
    Update,
}

fn update_waveforms(
    mut roots: Query<(
        Entity,
        &mut WaveformRuntime,
        &WaveformCursor,
        &WaveformOutputLength,
    )>,
    mut commands: Commands,
) -> Result {
    let mut failure = None;
    for (entity, mut runtime, cursor, output_len) in &mut roots {
        let was_running = runtime.is_running();
        let result = runtime.update_cursor(cursor, output_len.0);
        if was_running && !runtime.is_running() {
            commands.trigger(crate::WaveformRunningChanged {
                entity,
                running: false,
            });
        }
        if let Err(error) = result
            && failure.is_none()
        {
            failure = Some(BevyError::error(format!("Waveform {entity:?}: {error}")));
        }
    }
    failure.map_or(Ok(()), Err)
}

fn classify(current: &Range<u64>, target: &Range<u64>) -> WaveformUpdateKind {
    if current == target {
        WaveformUpdateKind::NoOp
    } else if target.end > current.end
        && target.start >= current.start
        && (target.start < current.end || current.is_empty() && target.start == current.start)
    {
        WaveformUpdateKind::IncrementalRead
    } else {
        WaveformUpdateKind::FullRead
    }
}

impl WaveformRuntime {
    pub fn new(
        config: WaveformConfig,
        source: Arc<dyn WaveformSource>,
        manager: &JobManager,
    ) -> Result<Self, BevyError> {
        Self::with_reducer(config, source, Box::<MinMaxReducer>::default(), manager)
    }

    pub(crate) fn with_reducer(
        config: WaveformConfig,
        source: Arc<dyn WaveformSource>,
        reducer: Box<dyn WaveformReducer>,
        manager: &JobManager,
    ) -> Result<Self, BevyError> {
        let mut runtime = Self::empty_with_reducer(config, reducer, manager)?;
        runtime.replace_session(source)?;
        Ok(runtime)
    }

    pub fn empty(config: WaveformConfig, manager: &JobManager) -> Result<Self, BevyError> {
        Self::empty_with_reducer(config, Box::<MinMaxReducer>::default(), manager)
    }

    fn empty_with_reducer(
        config: WaveformConfig,
        reducer: Box<dyn WaveformReducer>,
        manager: &JobManager,
    ) -> Result<Self, BevyError> {
        config.validate()?;
        Ok(Self {
            manager: manager.clone(),
            config,
            source: None,
            source_reduced: false,
            metadata: None,
            generation: 0,
            running: false,
            preview: false,
            rebuild: true,
            duration_refresh: false,
            requested_position: None,
            reducer,
            algorithm: default(),
            algorithm_refresh: false,
            ring: Arc::new(PlanarRing::new(0, 0)?),
            pending_reduction: None,
            reduction_generation: 0,
            staging: PlanarBuffer::default(),
            viewport: 0..0,
            reduced: Vec::new(),
            output_len: 0,
            revision: 0,
            stats: WaveformUpdateStats::default(),
            failure: FailureState::default(),
            last_error: None,
            pending_request: None,
            reader_id: NEXT_REQUEST.fetch_add(1, Ordering::Relaxed),
            target_generation: 0,
        })
    }

    pub(crate) fn replace_session(
        &mut self,
        source: Arc<dyn WaveformSource>,
    ) -> Result<(), BevyError> {
        let metadata = source.metadata().map_err(|error| {
            widgetry_error!(%error, "Waveform 会话元信息读取失败");
            BevyError::error(error.to_string())
        })?;
        metadata.validate()?;
        let capacity = self
            .config
            .capacity_frames(metadata.sample_rate, metadata.channel_count)?;
        let source_reduced = source.provides_reduced();
        let ring = if source_reduced {
            PlanarRing::new(0, 0)?
        } else {
            PlanarRing::new(metadata.channel_count, capacity)?
        };
        self.cancel_reduction()?;
        if let Some(previous) = &self.source {
            previous.cancel_reads(self.reader_id);
        }
        self.source_reduced = source_reduced;
        self.running = matches!(metadata.kind, crate::WaveformSessionKind::Live);
        self.source = Some(source);
        self.metadata = Some(metadata);
        self.ring = Arc::new(ring);
        self.reset_display(0..capacity as u64);
        Ok(())
    }

    fn reset_display(&mut self, viewport: Range<u64>) {
        self.reduction_generation = self.reduction_generation.wrapping_add(1);
        self.viewport = viewport;
        self.staging = PlanarBuffer::default();
        self.reduced.clear();
        self.rebuild = true;
        self.duration_refresh = false;
        self.requested_position = None;
        self.preview = false;
        self.stats = WaveformUpdateStats::default();
        self.failure = FailureState::default();
        self.last_error = None;
        self.pending_request = None;
        self.target_generation = 0;
        self.revision = self.revision.wrapping_add(1);
        self.generation = self.generation.wrapping_add(1);
    }

    pub(crate) fn clear_session(&mut self) -> Result<bool, BevyError> {
        if self.source.is_none() {
            return Ok(false);
        }
        self.cancel_reduction()?;
        self.ring = Arc::new(PlanarRing::new(0, 0)?);
        if let Some(source) = &self.source {
            source.cancel_reads(self.reader_id);
        }
        self.source_reduced = false;
        self.source = None;
        self.metadata = None;
        self.running = false;
        self.reset_display(0..0);
        Ok(true)
    }

    pub fn metadata(&self) -> Option<&crate::WaveformMetadata> {
        self.metadata.as_ref()
    }
    pub fn channel_count(&self) -> usize {
        self.metadata.as_ref().map_or(0, |data| data.channel_count)
    }
    pub fn session_generation(&self) -> u64 {
        self.generation
    }
    pub fn is_running(&self) -> bool {
        self.running
    }
    pub(crate) fn is_preview(&self) -> bool {
        self.preview
    }
    pub(crate) fn set_preview(&mut self, preview: bool) -> Result {
        if self.preview != preview {
            self.cancel_reduction()?;
            self.preview = preview;
            self.rebuild = true;
            self.pending_request = None;
            self.reduction_generation = self.reduction_generation.wrapping_add(1);
        }
        Ok(())
    }
    fn density(&self, output_len: usize) -> usize {
        if self.preview {
            output_len.div_ceil(4)
        } else {
            output_len
        }
    }
    pub fn algorithm(&self) -> crate::WaveformAlgorithm {
        self.algorithm
    }
    pub(crate) fn set_algorithm(&mut self, algorithm: crate::WaveformAlgorithm) -> Result<bool> {
        if self.algorithm == algorithm {
            return Ok(false);
        }
        self.cancel_reduction()?;
        self.algorithm = algorithm;
        self.reduction_generation = self.reduction_generation.wrapping_add(1);
        self.reducer = algorithm.reducer();
        self.rebuild = true;
        self.algorithm_refresh = true;
        self.pending_request = None;
        Ok(true)
    }
    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub(crate) fn set_visible_duration(
        &mut self,
        visible_duration_ms: u32,
    ) -> Result<bool, BevyError> {
        let config = WaveformConfig {
            visible_duration_ms,
            ..self.config.clone()
        };
        config.validate()?;
        if let Some(metadata) = &self.metadata {
            config.capacity_frames(metadata.sample_rate, metadata.channel_count)?;
        }
        if self.config == config {
            return Ok(false);
        }
        self.cancel_reduction()?;
        self.config = config;
        self.reduction_generation = self.reduction_generation.wrapping_add(1);
        self.duration_refresh = self.metadata.is_some();
        self.rebuild = true;
        self.pending_request = None;
        self.staging = PlanarBuffer::default();
        Ok(true)
    }
    pub(crate) fn set_amplitude(&mut self, amplitude: crate::WaveformAmplitude) -> bool {
        if self.config.amplitude_range == amplitude {
            return false;
        }
        self.config.amplitude_range = amplitude;
        true
    }

    pub(crate) fn set_running(&mut self, running: bool) -> Result<bool, BevyError> {
        if self.metadata.is_none() {
            widgetry_error!("Waveform 尚无数据源会话");
            return Err(BevyError::error("Waveform has no session"));
        }
        if self.running == running {
            return Ok(false);
        }
        self.cancel_reduction()?;
        self.running = running;
        self.reduction_generation = self.reduction_generation.wrapping_add(1);
        if !running {
            self.pending_request = None;
            if let Some(source) = &self.source {
                source.cancel_reads(self.reader_id);
            }
        }
        Ok(true)
    }

    pub(crate) fn request_position(&mut self, position: std::time::Duration) -> Result {
        self.cancel_reduction()?;
        self.requested_position = Some(position);
        self.duration_refresh = true;
        self.pending_request = None;
        self.reduction_generation = self.reduction_generation.wrapping_add(1);
        Ok(())
    }

    pub fn update(
        &mut self,
        position: std::time::Duration,
        output_len: usize,
    ) -> Result<WaveformUpdateStats, BevyError> {
        self.update_at(position, output_len, false)
    }

    fn update_cursor(
        &mut self,
        cursor: &WaveformCursor,
        output_len: usize,
    ) -> Result<WaveformUpdateStats, BevyError> {
        if self.target_generation != cursor.seek_generation {
            self.cancel_reduction()?;
            self.pending_request = None;
            self.target_generation = cursor.seek_generation;
        }
        self.update_at(cursor.position, output_len, true)
    }

    fn update_at(
        &mut self,
        position: std::time::Duration,
        output_len: usize,
        following: bool,
    ) -> Result<WaveformUpdateStats, BevyError> {
        if (self.metadata.is_none()
            || !self.running && !self.duration_refresh && !self.algorithm_refresh)
            && self.pending_reduction.is_none()
        {
            self.stats = WaveformUpdateStats::default();
            return Ok(self.stats);
        }
        let result = self.try_update(position, self.density(output_len), following);
        if result
            .as_ref()
            .is_ok_and(|stats| stats.kind == WaveformUpdateKind::Pending)
        {
            return result;
        }
        if result.is_err() {
            self.duration_refresh = false;
            self.requested_position = None;
            self.algorithm_refresh = false;
            self.pending_request = None;
            if self.metadata.as_ref().is_some_and(|metadata| {
                matches!(metadata.kind, crate::WaveformSessionKind::Replay { .. })
            }) {
                self.running = false;
            }
        }
        self.last_error = result.as_ref().err().map(ToString::to_string);
        self.failure.observe(
            result,
            |error| widgetry_error!(%error, "Waveform 更新失败，保留已提交数据"),
            || widgetry_info!("Waveform 数据读取恢复正常"),
        )
    }

    fn try_update(
        &mut self,
        position: std::time::Duration,
        output_len: usize,
        following: bool,
    ) -> Result<WaveformUpdateStats, BevyError> {
        if let Some(stats) = self.poll_reduction(position, output_len, following)? {
            return Ok(stats);
        }
        if self.source_reduced {
            return self.update_reduced(position, output_len, following);
        }
        let Some(metadata) = &self.metadata else {
            self.stats = WaveformUpdateStats::default();
            return Ok(self.stats);
        };
        let mut end = if let Some(position) = self.requested_position {
            sample_boundary(position, metadata.sample_rate)?
        } else if self.running {
            sample_boundary(position, metadata.sample_rate)?
        } else {
            self.ring.range.end
        };
        if let crate::WaveformSessionKind::Replay { total_frames } = metadata.kind {
            end = end.min(total_frames);
        }
        if (self.rebuild || self.output_len != output_len)
            && !self.duration_refresh
            && output_len > 0
            && (end == self.ring.range.end || following && end > self.ring.range.end)
            && (self.ring.range.end - self.ring.range.start) as usize * self.channel_count()
                >= 262_144
        {
            self.start_reduction(
                self.ring.clone(),
                self.viewport.clone(),
                output_len,
                WaveformUpdateStats::default(),
            )?;
            return Ok(self.pending_stats());
        }
        if following
            && let Some(pending) = &self.pending_request
            && pending.output_len == output_len
            && end >= pending.range.end
        {
            end = pending.range.end;
        }
        let unit = metadata.unit;
        let capacity = self
            .config
            .capacity_frames(metadata.sample_rate, metadata.channel_count)?;
        let target = end.saturating_sub(capacity as u64)..end;
        let viewport = target.start..end.max(capacity as u64);
        let kind = if capacity != self.ring.capacity()
            || (self.rebuild || self.output_len != output_len)
                && output_len > 0
                && capacity * metadata.channel_count >= 262_144
        {
            WaveformUpdateKind::FullRead
        } else {
            classify(&self.ring.range, &target)
        };
        let appended = if kind == WaveformUpdateKind::IncrementalRead {
            self.ring.range.end..target.end
        } else if kind == WaveformUpdateKind::FullRead {
            target.clone()
        } else {
            end..end
        };
        let frames = (appended.end - appended.start) as usize;
        let channels = metadata.channel_count;
        let mut replacement = None;
        if kind != WaveformUpdateKind::NoOp && frames > 0 {
            let request = match self.pending_request.take() {
                Some(request)
                    if request.session_generation == self.generation
                        && request.range == appended
                        && request.viewport == viewport
                        && request.output_len == output_len =>
                {
                    request
                }
                _ => crate::WaveformReadRequest {
                    id: NEXT_REQUEST.fetch_add(1, Ordering::Relaxed),
                    reader_id: self.reader_id,
                    session_generation: self.generation,
                    target_generation: self.target_generation,
                    range: appended.clone(),
                    viewport: viewport.clone(),
                    output_len,
                    algorithm: self.algorithm,
                },
            };
            self.pending_request = Some(request.clone());
            self.staging.prepare(channels, frames)?;
            let source = self
                .source
                .as_ref()
                .ok_or_else(|| BevyError::error("Waveform session source missing"))?;
            match source
                .read(&request, &mut self.staging)
                .map_err(|error| BevyError::error(error.to_string()))?
            {
                crate::WaveformReadStatus::Ready(reply) if reply == request => {}
                _ => {
                    self.stats = WaveformUpdateStats {
                        kind: WaveformUpdateKind::Pending,
                        ..default()
                    };
                    return Ok(self.stats);
                }
            }
            self.staging
                .validate(channels, frames)
                .map_err(|error| BevyError::error(error.to_string()))?;
            self.staging.convert_to_volts(unit);
            if kind == WaveformUpdateKind::FullRead
                || self.ring.range.is_empty() && frames * channels >= 262_144
            {
                replacement = Some(Arc::new(PlanarRing::from_staging(
                    std::mem::take(&mut self.staging),
                    target,
                    capacity,
                )?));
            } else {
                Arc::get_mut(&mut self.ring)
                    .ok_or_else(|| {
                        widgetry_error!("Waveform 读取缓存仍被降采样任务占用");
                        BevyError::error("Waveform ring still borrowed by reduction")
                    })?
                    .append(target, &self.staging);
                if frames > capacity / 4 {
                    self.staging = PlanarBuffer::default();
                }
            }
            self.pending_request = None;
        } else {
            self.pending_request = None;
            if kind == WaveformUpdateKind::FullRead {
                replacement = Some(Arc::new(PlanarRing::new(channels, capacity)?));
            }
        }
        let changed_width = self.output_len != output_len;
        let rebuild = self.rebuild || changed_width || kind == WaveformUpdateKind::FullRead;
        let next_ring = replacement.as_ref().unwrap_or(&self.ring);
        if rebuild
            && output_len > 0
            && (next_ring.range.end - next_ring.range.start) as usize * channels >= 262_144
        {
            let ring = next_ring.clone();
            self.start_reduction(
                ring,
                viewport,
                output_len,
                WaveformUpdateStats {
                    kind,
                    read_frames: frames,
                    ..default()
                },
            )?;
            return Ok(self.pending_stats());
        }
        if let Some(ring) = replacement {
            self.ring = ring;
        }
        let mut reduction = ReductionStats::default();
        if kind != WaveformUpdateKind::NoOp || changed_width || self.rebuild {
            self.viewport = viewport;
            reduction = self.reduce(
                output_len,
                appended,
                self.rebuild || changed_width || kind == WaveformUpdateKind::FullRead,
            );
            self.rebuild = false;
            self.output_len = output_len;
            self.revision = self.revision.wrapping_add(1);
        }
        self.stats = WaveformUpdateStats {
            kind,
            read_frames: frames,
            reduction,
        };
        self.duration_refresh = false;
        self.requested_position = None;
        self.algorithm_refresh = false;
        Ok(self.stats)
    }

    pub(crate) fn resize_reduction(&mut self, output_len: usize) -> Result {
        let output_len = self.density(output_len);
        if self.output_len == output_len {
            return Ok(());
        }
        if self.source_reduced {
            self.duration_refresh = true;
            return Ok(());
        }
        if self.pending_reduction.is_some() {
            self.cancel_reduction()?;
            return Ok(());
        }
        if output_len > 0
            && (self.ring.range.end - self.ring.range.start) as usize * self.channel_count()
                >= 262_144
        {
            let result = self.start_reduction(
                self.ring.clone(),
                self.viewport.clone(),
                output_len,
                WaveformUpdateStats::default(),
            );
            return match result {
                Ok(()) => Ok(()),
                Err(error) => self.failure.observe(
                    Err(error),
                    |error| widgetry_error!(%error, "Waveform 后台重建提交失败"),
                    || widgetry_info!("Waveform 后台重建恢复正常"),
                ),
            };
        }
        self.stats.reduction =
            self.reduce(output_len, self.ring.range.end..self.ring.range.end, true);
        self.output_len = output_len;
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    fn pending_stats(&mut self) -> WaveformUpdateStats {
        self.stats = WaveformUpdateStats {
            kind: WaveformUpdateKind::Pending,
            ..default()
        };
        self.stats
    }

    fn start_reduction(
        &mut self,
        ring: Arc<PlanarRing>,
        viewport: Range<u64>,
        output_len: usize,
        stats: WaveformUpdateStats,
    ) -> Result {
        if self.pending_reduction.is_some() {
            return Err(BevyError::error("Waveform already has an active reduction"));
        }
        let worker_ring = ring.clone();
        let worker_viewport = viewport.clone();
        let reducer = std::mem::replace(&mut self.reducer, self.algorithm.reducer());
        // The business handoff keeps exclusive reducer state outside the broadcast queue.
        // It also survives cancellation before the Manager calls the execution closure.
        let handoff = Arc::new(Mutex::new(Some(ReductionResult {
            reducer,
            reduced: Vec::new(),
            stats: Default::default(),
        })));
        let worker_handoff = handoff.clone();
        AsyncComputeTaskPool::get_or_init(TaskPool::new);
        let submitted = self.manager.spawn_cpu::<(), _, _, String>(
            JobOptions {
                label: Some("Waveform reduction".into()),
                ..default()
            },
            move |context| async move {
                if context.is_cancelled() {
                    context.acknowledge_cancel();
                    return Ok(());
                }
                let mut result = worker_handoff
                    .lock()
                    .map_err(|error| error.to_string())?
                    .take()
                    .ok_or_else(|| "Waveform reducer handoff empty".to_string())?;
                let views: Vec<_> = (0..worker_ring.channel_count())
                    .filter_map(|i| worker_ring.channel(i))
                    .collect();
                result.stats = result.reducer.reduce(
                    ReductionInput {
                        channels: &views,
                        buffered: worker_ring.range.clone(),
                        viewport: worker_viewport,
                        appended: worker_ring.range.clone(),
                        output_len,
                        rebuild: true,
                    },
                    &mut result.reduced,
                );
                *worker_handoff.lock().map_err(|error| error.to_string())? = Some(result);
                // A completed calculation remains Completed after a late, unacknowledged cancellation.
                match context.send(()).await {
                    Ok(()) | Err(SendError::Canceled) => Ok(()),
                    Err(error) => Err(error.to_string()),
                }
            },
        );
        let (job, receiver) = match submitted {
            Ok(submitted) => submitted,
            Err(error) => {
                if let Some(result) = handoff
                    .lock()
                    .map_err(|error| BevyError::error(error.to_string()))?
                    .take()
                {
                    self.reducer = result.reducer;
                }
                return Err(BevyError::error(error.to_string()));
            }
        };
        self.pending_reduction = Some(PendingReduction {
            job,
            receiver,
            handoff,
            received: false,
            algorithm: self.algorithm,
            ring,
            viewport,
            output_len,
            generation: self.reduction_generation,
            target_generation: self.target_generation,
            stats,
        });
        Ok(())
    }

    fn poll_reduction(
        &mut self,
        position: std::time::Duration,
        output_len: usize,
        following: bool,
    ) -> Result<Option<WaveformUpdateStats>, BevyError> {
        let Some(pending) = &mut self.pending_reduction else {
            return Ok(None);
        };
        let outcome = match pending.receiver.try_recv() {
            Ok(JobMessage::Item(_)) => {
                pending.received = true;
                return Ok(Some(self.pending_stats()));
            }
            Ok(JobMessage::Finished(outcome)) => outcome,
            Err(TryRecvError::Empty) => return Ok(Some(self.pending_stats())),
            Err(error) => return Err(BevyError::error(error.to_string())),
        };
        let pending = self
            .pending_reduction
            .take()
            .ok_or_else(|| BevyError::error("Waveform reduction task missing"))?;
        match self.manager.remove(pending.job) {
            Ok(_) | Err(JobAccessError::NotFound) => {}
            Err(error) => return Err(BevyError::error(error.to_string())),
        }
        let result = pending
            .handoff
            .lock()
            .map_err(|error| BevyError::error(error.to_string()))?
            .take();
        // Returned reducer caches describe the worker's target, not the committed snapshot.
        // Only accepting that target below makes incremental updates safe again.
        self.rebuild = true;
        let reduced = match result {
            Some(result) => {
                if pending.algorithm == self.algorithm {
                    self.reducer = result.reducer;
                }
                Some((result.reduced, result.stats))
            }
            None => None,
        };
        match outcome {
            JobOutcome::Failed(error) => return Err(BevyError::error(error.to_string())),
            JobOutcome::Canceled => {
                self.rebuild = true;
                return Ok(None);
            }
            JobOutcome::Completed if !pending.received => {
                self.rebuild = true;
                return Ok(None);
            }
            JobOutcome::Completed => {}
        }
        let (reduced, reduction_stats) = reduced
            .ok_or_else(|| BevyError::error("Waveform completed without returning its reducer"))?;
        let end = self
            .metadata
            .as_ref()
            .map(|meta| {
                sample_boundary(position, meta.sample_rate).map(|end| match meta.kind {
                    crate::WaveformSessionKind::Live => end,
                    crate::WaveformSessionKind::Replay { total_frames } => end.min(total_frames),
                })
            })
            .transpose()?
            .unwrap_or(0);
        if pending.generation != self.reduction_generation
            || pending.target_generation != self.target_generation
            || pending.output_len != output_len
            || self.running
                && (end < pending.ring.range.end || !following && end != pending.ring.range.end)
        {
            return Ok(None);
        }
        self.ring = pending.ring;
        self.viewport = pending.viewport;
        self.output_len = pending.output_len;
        self.reduced = reduced;
        self.stats = WaveformUpdateStats {
            reduction: reduction_stats,
            ..pending.stats
        };
        self.revision = self.revision.wrapping_add(1);
        self.rebuild = false;
        self.algorithm_refresh = false;
        self.duration_refresh = false;
        self.requested_position = None;
        Ok(Some(self.stats))
    }

    pub fn pending_reduction_job(&self) -> Option<JobId> {
        self.pending_reduction.as_ref().map(|pending| pending.job)
    }

    fn cancel_reduction(&self) -> Result {
        if let Some(pending) = &self.pending_reduction {
            match self.manager.cancel(pending.job) {
                Ok(_) | Err(JobAccessError::NotFound) => {}
                Err(error) => {
                    widgetry_error!(%error, "取消 Waveform 后台重建失败");
                    return Err(BevyError::error(error.to_string()));
                }
            }
        }
        Ok(())
    }

    fn reduce(&mut self, output_len: usize, appended: Range<u64>, rebuild: bool) -> ReductionStats {
        if output_len == 0 {
            return ReductionStats::default();
        }
        let views: Vec<_> = (0..self.channel_count())
            .filter_map(|index| self.ring.channel(index))
            .collect();
        self.reducer.reduce(
            ReductionInput {
                channels: &views,
                buffered: self.ring.range.clone(),
                viewport: self.viewport.clone(),
                appended,
                output_len,
                rebuild,
            },
            &mut self.reduced,
        )
    }

    pub fn config(&self) -> &WaveformConfig {
        &self.config
    }

    pub fn viewport_range(&self) -> Range<u64> {
        self.viewport.clone()
    }

    pub fn buffered_range(&self) -> Range<u64> {
        self.ring.range.clone()
    }

    pub fn channel(&self, index: usize) -> Option<ChannelView<'_>> {
        self.ring.channel(index)
    }

    pub fn reduced_channels(&self) -> &[ReducedChannel] {
        &self.reduced
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn stats(&self) -> WaveformUpdateStats {
        self.stats
    }

    pub fn working_set_bytes(&self) -> (usize, usize, usize) {
        let reduced = self
            .reduced
            .iter()
            .map(|channel| match channel {
                ReducedChannel::Polyline(points) => {
                    points.capacity() * size_of::<crate::WaveformPoint>()
                }
                ReducedChannel::Envelope(spans) => {
                    spans.capacity() * size_of::<crate::WaveformSpan>()
                }
            })
            .sum();
        (
            self.ring.allocated_samples() * size_of::<f32>() + self.ring.missing_bytes(),
            self.staging.allocated_samples() * size_of::<f32>() + self.staging.missing_bytes(),
            reduced,
        )
    }
}

impl WaveformRuntime {
    fn update_reduced(
        &mut self,
        position: std::time::Duration,
        output_len: usize,
        following: bool,
    ) -> Result<WaveformUpdateStats, BevyError> {
        let Some(metadata) = &self.metadata else {
            return Ok(WaveformUpdateStats::default());
        };
        if output_len == 0 {
            return Ok(self.pending_stats());
        }
        let mut end = if let Some(position) = self.requested_position {
            sample_boundary(position, metadata.sample_rate)?
        } else if self.running {
            sample_boundary(position, metadata.sample_rate)?
        } else {
            self.ring.range.end
        };
        if let crate::WaveformSessionKind::Replay { total_frames } = metadata.kind {
            end = end.min(total_frames);
        }
        if following
            && let Some(request) = &self.pending_request
            && request.output_len == output_len
            && end >= request.range.end
        {
            end = request.range.end;
        }
        let capacity =
            self.config
                .capacity_frames(metadata.sample_rate, metadata.channel_count)? as u64;
        let range = end.saturating_sub(capacity)..end;
        let viewport = range.start..end.max(capacity);
        if !self.rebuild && self.output_len == output_len && self.ring.range == range {
            self.stats = WaveformUpdateStats::default();
            self.duration_refresh = false;
            self.requested_position = None;
            self.algorithm_refresh = false;
            return Ok(self.stats);
        }
        let request = match self.pending_request.take() {
            Some(request)
                if request.range == range
                    && request.viewport == viewport
                    && request.output_len == output_len
                    && request.algorithm == self.algorithm =>
            {
                request
            }
            _ => crate::WaveformReadRequest {
                id: NEXT_REQUEST.fetch_add(1, Ordering::Relaxed),
                reader_id: self.reader_id,
                session_generation: self.generation,
                target_generation: self.target_generation,
                range: range.clone(),
                viewport: viewport.clone(),
                output_len,
                algorithm: self.algorithm,
            },
        };
        self.pending_request = Some(request.clone());
        let source = self
            .source
            .as_ref()
            .ok_or_else(|| BevyError::error("Waveform source missing"))?;
        let mut channels = if range.is_empty() {
            (0..metadata.channel_count)
                .map(|_| ReducedChannel::Polyline(Vec::new()))
                .collect()
        } else {
            match source
                .read_reduced(&request)
                .map_err(|e| BevyError::error(e.to_string()))?
            {
                crate::WaveformReducedReadStatus::Ready {
                    request: reply,
                    channels,
                } if reply == request => channels,
                _ => return Ok(self.pending_stats()),
            }
        };
        validate_reduced(&mut channels, &request, metadata)?;
        let mut ring = PlanarRing::new(0, 0)?;
        ring.range = range;
        self.ring = Arc::new(ring);
        self.viewport = viewport;
        self.reduced = channels;
        self.output_len = output_len;
        self.pending_request = None;
        self.rebuild = false;
        self.duration_refresh = false;
        self.requested_position = None;
        self.algorithm_refresh = false;
        self.revision = self.revision.wrapping_add(1);
        self.stats = WaveformUpdateStats {
            kind: WaveformUpdateKind::FullRead,
            ..default()
        };
        Ok(self.stats)
    }
}

fn validate_reduced(
    channels: &mut [ReducedChannel],
    request: &crate::WaveformReadRequest,
    metadata: &crate::WaveformMetadata,
) -> Result<(), BevyError> {
    let invalid = || BevyError::error("invalid reduced waveform result");
    if channels.len() != metadata.channel_count {
        return Err(invalid());
    }
    let factor = metadata.unit.volts();
    for channel in channels {
        match channel {
            ReducedChannel::Envelope(spans) => {
                if request.algorithm == crate::WaveformAlgorithm::Lttb {
                    return Err(invalid());
                }
                let mut end = request.range.start;
                for span in spans {
                    span.min *= factor;
                    span.max *= factor;
                    if span.samples.start < end
                        || span.samples.start >= span.samples.end
                        || span.samples.end > request.range.end
                        || !span.min.is_finite()
                        || !span.max.is_finite()
                        || span.min > span.max
                    {
                        return Err(invalid());
                    }
                    end = span.samples.end;
                }
            }
            ReducedChannel::Polyline(points) => {
                let mut previous = None;
                for point in points {
                    point.value *= factor;
                    if point.sample < request.range.start
                        || point.sample >= request.range.end
                        || previous.is_some_and(|p| p >= point.sample)
                        || previous.is_none() && !point.break_before
                        || !point.value.is_finite()
                    {
                        return Err(invalid());
                    }
                    previous = Some(point.sample);
                }
            }
        }
    }
    Ok(())
}

impl Drop for WaveformRuntime {
    fn drop(&mut self) {
        drop(self.cancel_reduction());
        if let Some(source) = &self.source {
            source.cancel_reads(self.reader_id);
        }
    }
}

impl Plugin for WaveformPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<JobManagerPlugin>() {
            app.add_plugins(JobManagerPlugin);
        }
        app.add_message::<bevy::window::RequestRedraw>()
            .add_systems(Update, update_waveforms.in_set(WaveformSystems::Update))
            .add_systems(Last, redraw_pending_reductions);
        widgetry_info!("WaveformPlugin 注册完成");
    }
}

fn redraw_pending_reductions(
    roots: Query<(&WaveformRuntime, &WaveformOutputLength)>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    // Layout can start a rebuild in PostUpdate, after the normal update has already run.
    // Keep reactive windows awake until the complete snapshot can be committed.
    if roots.iter().any(|(runtime, output)| {
        output.0 > 0
            && (runtime.pending_reduction.is_some()
                || runtime.pending_request.is_some()
                || runtime.source_reduced && runtime.duration_refresh)
    }) {
        redraw.write(bevy::window::RequestRedraw);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct SessionSource(bool);

    struct ControlledReducer {
        reducer: MinMaxReducer,
        calls: Arc<std::sync::atomic::AtomicUsize>,
        drops: Arc<std::sync::atomic::AtomicUsize>,
        gate: Option<(
            std::sync::mpsc::Sender<()>,
            Mutex<std::sync::mpsc::Receiver<()>>,
        )>,
        fail: bool,
    }
    impl Drop for ControlledReducer {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }
    impl WaveformReducer for ControlledReducer {
        fn reduce(
            &mut self,
            input: ReductionInput<'_>,
            output: &mut Vec<ReducedChannel>,
        ) -> ReductionStats {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if input.buffered.end - input.buffered.start >= 262_144 {
                assert!(!self.fail, "controlled reducer failure");
                if let Some((entered, gate)) = self.gate.take() {
                    entered.send(()).unwrap();
                    gate.lock().unwrap().recv().unwrap();
                }
            }
            self.reducer.reduce(input, output)
        }
    }

    fn settle(runtime: &mut WaveformRuntime, position: Duration, width: usize) {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while runtime.update(position, width).unwrap().kind == WaveformUpdateKind::Pending {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn canceled_inflight_rebuild_keeps_the_snapshot_and_returns_the_only_reducer_before_restarting()
    {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let drops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let (entered, ready) = std::sync::mpsc::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let manager = JobManager::default();
        let mut runtime = WaveformRuntime::with_reducer(
            WaveformConfig {
                visible_duration_ms: 10_000,
                ..default()
            },
            Arc::new(SessionSource(false)),
            Box::new(ControlledReducer {
                reducer: default(),
                calls: calls.clone(),
                drops: drops.clone(),
                gate: Some((entered, Mutex::new(gate))),
                fail: false,
            }),
            &manager,
        )
        .unwrap();
        runtime.update(Duration::from_secs(1), 1200).unwrap();
        let snapshot = runtime.reduced_channels().to_vec();
        let revision = runtime.revision();
        assert_eq!(
            runtime.update(Duration::from_secs(10), 600).unwrap().kind,
            WaveformUpdateKind::Pending
        );
        let old = runtime.pending_reduction_job().unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        runtime.set_preview(true).unwrap();
        runtime.request_position(Duration::from_secs(10)).unwrap();
        for _ in 0..3 {
            assert_eq!(
                runtime.update(Duration::from_secs(10), 1200).unwrap().kind,
                WaveformUpdateKind::Pending
            );
            assert_eq!(runtime.pending_reduction_job(), Some(old));
            assert_eq!(runtime.reduced_channels(), snapshot);
            assert_eq!(runtime.revision(), revision);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            manager.status(old).unwrap().state,
            bevy_widgetry_job::JobState::Stopping
        );
        release.send(()).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !manager.status(old).unwrap().state.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(manager.outcome(old).unwrap(), Some(JobOutcome::Completed));
        settle(&mut runtime, Duration::from_secs(10), 1200);
        assert_eq!(runtime.buffered_range(), 0..640_000);
        assert_eq!(runtime.output_len, 300);
        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(runtime);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn rejected_distant_rebuild_cannot_supply_caches_for_the_committed_window() {
        let (entered, ready) = std::sync::mpsc::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let manager = JobManager::default();
        let config = WaveformConfig {
            visible_duration_ms: 10_000,
            ..default()
        };
        let mut runtime = WaveformRuntime::with_reducer(
            config.clone(),
            Arc::new(SessionSource(false)),
            Box::new(ControlledReducer {
                reducer: default(),
                calls: Arc::default(),
                drops: Arc::default(),
                gate: Some((entered, Mutex::new(gate))),
                fail: false,
            }),
            &manager,
        )
        .unwrap();
        settle(&mut runtime, Duration::from_secs(1), 1200);
        let snapshot = runtime.reduced_channels().to_vec();
        runtime.update(Duration::from_secs(30), 1200).unwrap();
        ready.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(
            runtime.update(Duration::from_secs(1), 1200).unwrap().kind,
            WaveformUpdateKind::Pending
        );
        assert_eq!(runtime.reduced_channels(), snapshot);
        release.send(()).unwrap();
        settle(&mut runtime, Duration::from_secs(1), 1200);
        settle(&mut runtime, Duration::from_secs(2), 1200);

        let mut fresh =
            WaveformRuntime::new(config, Arc::new(SessionSource(false)), &manager).unwrap();
        settle(&mut fresh, Duration::from_secs(1), 1200);
        settle(&mut fresh, Duration::from_secs(2), 1200);
        assert_eq!(runtime.buffered_range(), fresh.buffered_range());
        assert_eq!(runtime.reduced_channels(), fresh.reduced_channels());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn failed_rebuild_reports_the_job_failure_without_clearing_the_committed_snapshot() {
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let drops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let manager = JobManager::default();
        let mut runtime = WaveformRuntime::with_reducer(
            WaveformConfig {
                visible_duration_ms: 10_000,
                ..default()
            },
            Arc::new(SessionSource(false)),
            Box::new(ControlledReducer {
                reducer: default(),
                calls,
                drops: drops.clone(),
                gate: None,
                fail: true,
            }),
            &manager,
        )
        .unwrap();
        runtime.update(Duration::from_secs(1), 1200).unwrap();
        let snapshot = runtime.reduced_channels().to_vec();
        let range = runtime.buffered_range();
        let revision = runtime.revision();
        runtime.update(Duration::from_secs(10), 600).unwrap();
        let id = runtime.pending_reduction_job().unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(matches!(
            manager.outcome(id).unwrap(),
            Some(JobOutcome::Failed(_))
        ));
        let error = runtime.update(Duration::from_secs(10), 1200).unwrap_err();
        assert!(error.to_string().contains("controlled reducer failure"));
        assert!(runtime.pending_reduction_job().is_none());
        assert_eq!(runtime.reduced_channels(), snapshot);
        assert_eq!(runtime.buffered_range(), range);
        assert_eq!(runtime.revision(), revision);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn finished_and_record_removal_do_not_commit_before_the_receiver_consumes_finished() {
        let manager = JobManager::default();
        let mut runtime = WaveformRuntime::new(
            WaveformConfig {
                visible_duration_ms: 10_000,
                ..default()
            },
            Arc::new(SessionSource(false)),
            &manager,
        )
        .unwrap();
        runtime.update(Duration::from_secs(1), 1200).unwrap();
        let snapshot = runtime.reduced_channels().to_vec();
        let revision = runtime.revision();
        runtime.update(Duration::from_secs(10), 600).unwrap();
        let id = runtime.pending_reduction_job().unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        manager.remove(id).unwrap();
        assert_eq!(
            runtime.update(Duration::from_secs(10), 600).unwrap().kind,
            WaveformUpdateKind::Pending
        );
        assert_eq!(runtime.revision(), revision);
        assert_eq!(runtime.reduced_channels(), snapshot);
        assert_eq!(
            runtime.update(Duration::from_secs(10), 600).unwrap().kind,
            WaveformUpdateKind::FullRead
        );
        assert_eq!(runtime.buffered_range(), 0..640_000);
        assert!(runtime.pending_reduction_job().is_none());
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn canceled_without_computation_restores_exclusive_state_even_with_a_shared_handoff() {
        AsyncComputeTaskPool::get_or_init(TaskPool::new);
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let drops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let manager = JobManager::default();
        let mut runtime = WaveformRuntime::with_reducer(
            WaveformConfig {
                visible_duration_ms: 10_000,
                ..default()
            },
            Arc::new(SessionSource(false)),
            Box::new(ControlledReducer {
                reducer: default(),
                calls: calls.clone(),
                drops: drops.clone(),
                gate: None,
                fail: false,
            }),
            &manager,
        )
        .unwrap();
        runtime.update(Duration::from_secs(1), 1200).unwrap();
        let reducer = std::mem::replace(&mut runtime.reducer, runtime.algorithm.reducer());
        let handoff = Arc::new(Mutex::new(Some(ReductionResult {
            reducer,
            reduced: vec![],
            stats: default(),
        })));
        let (job, receiver) = manager
            .spawn_cpu::<(), _, _, String>(Default::default(), |context| async move {
                context.cancelled().await;
                context.acknowledge_cancel();
                Ok(())
            })
            .unwrap();
        runtime.pending_reduction = Some(PendingReduction {
            job,
            receiver,
            handoff: handoff.clone(),
            received: false,
            algorithm: runtime.algorithm,
            ring: runtime.ring.clone(),
            viewport: runtime.viewport.clone(),
            output_len: runtime.output_len,
            generation: runtime.reduction_generation,
            target_generation: runtime.target_generation,
            stats: default(),
        });
        manager.cancel(job).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !manager.status(job).unwrap().state.is_finished() {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(manager.outcome(job).unwrap(), Some(JobOutcome::Canceled));
        runtime.update(Duration::from_secs(1), 1200).unwrap();
        assert!(handoff.lock().unwrap().is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        drop(runtime);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    impl WaveformSource for SessionSource {
        fn metadata(&self) -> Result<crate::WaveformMetadata, crate::WaveformReadError> {
            Ok(crate::WaveformMetadata {
                channel_count: 1,
                channel_labels: vec!["A".into()],
                sample_rate: 64_000,
                unit: crate::WaveformUnit::Volts,
                kind: if self.0 {
                    crate::WaveformSessionKind::Replay {
                        total_frames: 640_000,
                    }
                } else {
                    crate::WaveformSessionKind::Live
                },
            })
        }
        fn provides_reduced(&self) -> bool {
            self.0
        }
        fn read(
            &self,
            request: &crate::WaveformReadRequest,
            out: &mut PlanarBuffer,
        ) -> Result<crate::WaveformReadStatus, crate::WaveformReadError> {
            for channel in out.channels_mut() {
                channel.resize((request.range.end - request.range.start) as usize, 1.0);
            }
            Ok(crate::WaveformReadStatus::Ready(request.clone()))
        }
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn switching_to_reduced_session_cancels_an_inflight_raw_reduction() {
        let mut runtime = WaveformRuntime::new(
            WaveformConfig {
                visible_duration_ms: 10_000,
                ..default()
            },
            Arc::new(SessionSource(false)),
            &bevy_widgetry_job::JobManager::default(),
        )
        .unwrap();
        assert_eq!(
            runtime
                .update(std::time::Duration::from_secs(10), 1200)
                .unwrap()
                .kind,
            WaveformUpdateKind::Pending
        );
        assert!(runtime.pending_reduction.is_some());
        runtime
            .replace_session(Arc::new(SessionSource(true)))
            .unwrap();
        assert!(runtime.pending_reduction.is_some());
        assert!(!runtime.is_running());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while runtime.pending_reduction.is_some() {
            runtime.update(Duration::ZERO, 1200).unwrap();
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(runtime.working_set_bytes(), (0, 0, 0));
    }

    #[test]
    fn half_open_range_plans() {
        assert_eq!(classify(&(0..0), &(0..0)), WaveformUpdateKind::NoOp);
        assert_eq!(
            classify(&(0..0), &(0..4)),
            WaveformUpdateKind::IncrementalRead
        );
        assert_eq!(
            classify(&(0..4), &(0..8)),
            WaveformUpdateKind::IncrementalRead
        );
        assert_eq!(
            classify(&(0..8), &(3..11)),
            WaveformUpdateKind::IncrementalRead
        );
        assert_eq!(classify(&(3..11), &(0..8)), WaveformUpdateKind::FullRead);
        assert_eq!(classify(&(3..11), &(11..19)), WaveformUpdateKind::FullRead);
    }
}
