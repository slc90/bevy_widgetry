use crate::source::PlanarRing;
use crate::{
    ChannelView, MinMaxReducer, PlanarBuffer, ReducedChannel, ReductionInput, ReductionStats,
    WaveformConfig, WaveformCursor, WaveformReducer, WaveformSource, sample_boundary,
};
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, Task, TaskPool, block_on, poll_once};
use bevy_widgetry_core::diagnostics::FailureState;
use bevy_widgetry_internal_log::{widgetry_error, widgetry_info};
use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_REQUEST: AtomicU64 = AtomicU64::new(1);

#[derive(Component)]
pub struct WaveformRuntime {
    config: WaveformConfig,
    source: Option<Arc<dyn WaveformSource>>,
    metadata: Option<crate::WaveformMetadata>,
    generation: u64,
    running: bool,
    rebuild: bool,
    duration_refresh: bool,
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
    task: Task<ReductionResult>,
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
    pub fn new(config: WaveformConfig, source: Arc<dyn WaveformSource>) -> Result<Self, BevyError> {
        Self::with_reducer(config, source, Box::<MinMaxReducer>::default())
    }

    pub(crate) fn with_reducer(
        config: WaveformConfig,
        source: Arc<dyn WaveformSource>,
        reducer: Box<dyn WaveformReducer>,
    ) -> Result<Self, BevyError> {
        let mut runtime = Self::empty_with_reducer(config, reducer)?;
        runtime.replace_session(source)?;
        Ok(runtime)
    }

    pub fn empty(config: WaveformConfig) -> Result<Self, BevyError> {
        Self::empty_with_reducer(config, Box::<MinMaxReducer>::default())
    }

    fn empty_with_reducer(
        config: WaveformConfig,
        reducer: Box<dyn WaveformReducer>,
    ) -> Result<Self, BevyError> {
        config.validate()?;
        Ok(Self {
            config,
            source: None,
            metadata: None,
            generation: 0,
            running: false,
            rebuild: true,
            duration_refresh: false,
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
        let ring = PlanarRing::new(metadata.channel_count, capacity)?;
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
        self.ring = Arc::new(PlanarRing::new(0, 0)?);
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
    pub fn algorithm(&self) -> crate::WaveformAlgorithm {
        self.algorithm
    }
    pub(crate) fn set_algorithm(&mut self, algorithm: crate::WaveformAlgorithm) -> bool {
        if self.algorithm == algorithm {
            return false;
        }
        self.algorithm = algorithm;
        self.reduction_generation = self.reduction_generation.wrapping_add(1);
        self.reducer = algorithm.reducer();
        self.rebuild = true;
        self.algorithm_refresh = true;
        self.pending_request = None;
        true
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
        self.running = running;
        self.reduction_generation = self.reduction_generation.wrapping_add(1);
        Ok(true)
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
        let result = self.try_update(position, output_len, following);
        if result
            .as_ref()
            .is_ok_and(|stats| stats.kind == WaveformUpdateKind::Pending)
        {
            return result;
        }
        if result.is_err() {
            self.duration_refresh = false;
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
        let Some(metadata) = &self.metadata else {
            self.stats = WaveformUpdateStats::default();
            return Ok(self.stats);
        };
        let mut end = if self.running {
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
            );
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
            );
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
        self.algorithm_refresh = false;
        Ok(self.stats)
    }

    pub(crate) fn resize_reduction(&mut self, output_len: usize) {
        if self.output_len == output_len {
            return;
        }
        if self.pending_reduction.is_some() {
            return;
        }
        if output_len > 0
            && (self.ring.range.end - self.ring.range.start) as usize * self.channel_count()
                >= 262_144
        {
            self.start_reduction(
                self.ring.clone(),
                self.viewport.clone(),
                output_len,
                WaveformUpdateStats::default(),
            );
            return;
        }
        self.stats.reduction =
            self.reduce(output_len, self.ring.range.end..self.ring.range.end, true);
        self.output_len = output_len;
        self.revision = self.revision.wrapping_add(1);
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
    ) {
        let worker_ring = ring.clone();
        let worker_viewport = viewport.clone();
        let mut reducer = std::mem::replace(&mut self.reducer, self.algorithm.reducer());
        let task = AsyncComputeTaskPool::get_or_init(TaskPool::new).spawn(async move {
            let views: Vec<_> = (0..worker_ring.channel_count())
                .filter_map(|i| worker_ring.channel(i))
                .collect();
            let mut reduced = Vec::new();
            let stats = reducer.reduce(
                ReductionInput {
                    channels: &views,
                    buffered: worker_ring.range.clone(),
                    viewport: worker_viewport,
                    appended: worker_ring.range.clone(),
                    output_len,
                    rebuild: true,
                },
                &mut reduced,
            );
            ReductionResult {
                reducer,
                reduced,
                stats,
            }
        });
        self.pending_reduction = Some(PendingReduction {
            task,
            ring,
            viewport,
            output_len,
            generation: self.reduction_generation,
            target_generation: self.target_generation,
            stats,
        });
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
        let Some(result) = block_on(poll_once(&mut pending.task)) else {
            return Ok(Some(self.pending_stats()));
        };
        let pending = self
            .pending_reduction
            .take()
            .ok_or_else(|| BevyError::error("Waveform reduction task missing"))?;
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
        self.reducer = result.reducer;
        self.reduced = result.reduced;
        self.stats = WaveformUpdateStats {
            reduction: result.stats,
            ..pending.stats
        };
        self.revision = self.revision.wrapping_add(1);
        self.rebuild = false;
        self.algorithm_refresh = false;
        self.duration_refresh = false;
        Ok(Some(self.stats))
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

impl Plugin for WaveformPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<bevy::window::RequestRedraw>()
            .add_systems(Update, update_waveforms.in_set(WaveformSystems::Update))
            .add_systems(Last, redraw_pending_reductions);
        widgetry_info!("WaveformPlugin 注册完成");
    }
}

fn redraw_pending_reductions(
    roots: Query<&WaveformRuntime>,
    mut redraw: MessageWriter<bevy::window::RequestRedraw>,
) {
    // Layout can start a rebuild in PostUpdate, after the normal update has already run.
    // Keep reactive windows awake until the complete snapshot can be committed.
    if roots
        .iter()
        .any(|runtime| runtime.pending_reduction.is_some())
    {
        redraw.write(bevy::window::RequestRedraw);
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;

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
