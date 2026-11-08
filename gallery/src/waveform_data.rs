const FREQUENCY_CHANNEL_CYCLE: u64 = 7;
const PHASE_CHANNEL_CYCLE: u64 = 4;
const SQUARE_DUTY_CYCLE: f32 = 0.5;
const SQUARE_AMPLITUDE: f32 = 0.8;
const TRIANGLE_AMPLITUDE: f32 = 0.6;
const SPIKE_FRAME_OFFSET: u64 = 11;
const SPIKE_AMPLITUDE: f32 = 1.5;
const AMPLITUDE_CHANNEL_CYCLE: usize = 4;
const AMPLITUDE_CHANNEL_STEP: f32 = 0.15;

const MAGIC: &[u8; 4] = b"WFRM";
const HEADER_LEN: usize = 24;
const WORD_LEN: usize = 4;
const VERSION_OFFSET: usize = 4;
const RATE_OFFSET: usize = 8;
const CHANNEL_COUNT_OFFSET: usize = 12;
const FRAME_COUNT_FIELD: std::ops::Range<usize> = 16..24;
const FORMAT_VERSION: u32 = 1;
const SAMPLE_LEN: usize = 4;

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use bevy_widgetry::waveform::{
    PlanarBuffer, WaveformMetadata, WaveformReadError, WaveformReadRequest, WaveformReadStatus,
    WaveformSessionKind, WaveformSource, WaveformUnit,
};
use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, RwLock};

#[derive(Asset, TypePath)]
pub(crate) struct ReplayAsset(pub Arc<ReplayData>);

pub(crate) struct ReplayData {
    pub rate: u32,
    pub frames: u64,
    pub channels: Vec<Vec<f32>>,
}

#[derive(Default, TypePath)]
pub(crate) struct ReplayLoader;

impl AssetLoader for ReplayLoader {
    type Asset = ReplayAsset;
    type Settings = ();
    type Error = bevy_widgetry::diagnostics::bevy::WidgetryEcsError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        context: &mut LoadContext<'_>,
    ) -> std::result::Result<ReplayAsset, Self::Error> {
        let result = async {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            parse(&bytes).map(|data| ReplayAsset(Arc::new(data)))
        }
        .await;
        result.map_err(|source| {
            bevy_widgetry::diagnostics::bevy::WidgetryEcsError::new(crate::diagnostics::failure(
                crate::diagnostics::GalleryError::Operation(source.into()),
                bevy_widgetry::diagnostics::ErrorFrame::new("gallery.waveform.load_asset")
                    .field("path", context.path()),
            ))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["wfrm"]
    }
}

fn parse(bytes: &[u8]) -> io::Result<ReplayData> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid WFRM payload");
    if bytes.len() < HEADER_LEN || &bytes[..MAGIC.len()] != MAGIC {
        return Err(invalid());
    }
    let word = |offset| -> io::Result<u32> {
        Ok(u32::from_le_bytes(
            bytes[offset..offset + WORD_LEN]
                .try_into()
                .map_err(|_| invalid())?,
        ))
    };
    let rate = word(RATE_OFFSET)?;
    let count = word(CHANNEL_COUNT_OFFSET)? as usize;
    let frames = u64::from_le_bytes(bytes[FRAME_COUNT_FIELD].try_into().map_err(|_| invalid())?);
    let length = usize::try_from(frames).map_err(|_| invalid())?;
    if word(VERSION_OFFSET)? != FORMAT_VERSION
        || rate == 0
        || count == 0
        || length == 0
        || count
            .checked_mul(length)
            .and_then(|n| n.checked_mul(SAMPLE_LEN))
            .and_then(|n| n.checked_add(HEADER_LEN))
            != Some(bytes.len())
    {
        return Err(invalid());
    }
    let mut channels = Vec::with_capacity(count);
    for channel in bytes[HEADER_LEN..].chunks_exact(length * SAMPLE_LEN) {
        let mut values = Vec::with_capacity(length);
        for sample in channel.as_chunks::<SAMPLE_LEN>().0 {
            let value = f32::from_le_bytes(*sample);
            if !value.is_finite() {
                return Err(invalid());
            }
            values.push(value);
        }
        channels.push(values);
    }
    Ok(ReplayData {
        rate,
        frames,
        channels,
    })
}

pub(crate) struct ReplaySource(pub Arc<ReplayData>, pub Vec<String>);

impl WaveformSource for ReplaySource {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            sample_rate: self.0.rate,
            channel_count: self.0.channels.len(),
            channel_labels: self.1.clone(),
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Replay {
                total_frames: self.0.frames,
            },
        })
    }

    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        let range = request.range.clone();
        let data = &self.0;
        if range.end > data.frames
            || range.start > range.end
            || out.channels().len() != data.channels.len()
        {
            return Err(bevy_widgetry::diagnostics::WidgetryFailure::new(
                bevy_widgetry::waveform::WaveformError::SourceRange {
                    request: Box::new(request.clone()),
                    available: 0..data.frames,
                    expected_channels: data.channels.len(),
                    actual_channels: out.channels().len(),
                },
                bevy_widgetry::diagnostics::ErrorFrame::new("gallery.waveform.replay.read"),
            )
            .into());
        }
        for (target, source) in out.channels_mut().iter_mut().zip(&data.channels) {
            target.extend_from_slice(&source[range.start as usize..range.end as usize]);
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

pub(crate) struct LiveStorage {
    pub channels: Vec<VecDeque<f32>>,
    pub end: u64,
    pub capacity: usize,
}

pub(crate) struct LiveSource(pub RwLock<LiveStorage>, u32, Vec<String>, bool);

impl LiveSource {
    pub fn new(labels: Vec<String>, capacity: usize, rate: u32, gaps: bool) -> Self {
        let channels = labels.len();
        Self(
            RwLock::new(LiveStorage {
                channels: (0..channels)
                    .map(|_| VecDeque::with_capacity(capacity))
                    .collect(),
                end: 0,
                capacity,
            }),
            rate,
            labels,
            gaps,
        )
    }

    pub fn produce(&self, target: u64, rate: u32, basic: bool) -> Result {
        let mut storage = self
            .0
            .write()
            .map_err(|_| crate::diagnostics::error("live producer lock poisoned"))?;
        let start = storage.end;
        let capacity = storage.capacity;
        for (channel, values) in storage.channels.iter_mut().enumerate() {
            let period = u64::from(rate) * (channel as u64 % FREQUENCY_CHANNEL_CYCLE + 1);
            for frame in start..target {
                let phase = ((frame
                    + period * (channel as u64 % PHASE_CHANNEL_CYCLE) / PHASE_CHANNEL_CYCLE)
                    % period) as f32
                    / period as f32;
                let value = if basic {
                    match channel {
                        0 => (phase * std::f32::consts::TAU).sin(),
                        1 => {
                            if phase < SQUARE_DUTY_CYCLE {
                                SQUARE_AMPLITUDE
                            } else {
                                -SQUARE_AMPLITUDE
                            }
                        }
                        2 => phase * 2.0 - 1.0,
                        _ => (1.0 - (phase * 2.0 - 1.0).abs() * 2.0) * TRIANGLE_AMPLITUDE,
                    }
                } else {
                    if frame % u64::from(rate) == SPIKE_FRAME_OFFSET + channel as u64 {
                        SPIKE_AMPLITUDE
                    } else {
                        (phase * 2.0 - 1.0)
                            * (1.0
                                - (channel % AMPLITUDE_CHANNEL_CYCLE) as f32
                                    * AMPLITUDE_CHANNEL_STEP)
                    }
                };
                if values.len() == capacity {
                    values.pop_front();
                }
                values.push_back(value);
            }
        }
        storage.end = target;
        Ok(())
    }
}

impl WaveformSource for LiveSource {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(WaveformMetadata {
            sample_rate: self.1,
            channel_count: self.2.len(),
            channel_labels: self.2.clone(),
            unit: WaveformUnit::Volts,
            kind: WaveformSessionKind::Live,
        })
    }

    fn read(
        &self,
        request: &WaveformReadRequest,
        out: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        let range = request.range.clone();
        let storage = self.0.read().map_err(|_| {
            WaveformReadError::external(
                LiveSourcePoisoned,
                bevy_widgetry::diagnostics::ErrorFrame::new("gallery.waveform.live.read"),
            )
        })?;
        if range.start > range.end
            || range.start < storage.end.saturating_sub(storage.capacity as u64)
            || range.end > storage.end
            || out.channels().len() != storage.channels.len()
        {
            return Err(bevy_widgetry::diagnostics::WidgetryFailure::new(
                bevy_widgetry::waveform::WaveformError::SourceRange {
                    request: Box::new(request.clone()),
                    available: storage.end.saturating_sub(storage.capacity as u64)..storage.end,
                    expected_channels: storage.channels.len(),
                    actual_channels: out.channels().len(),
                },
                bevy_widgetry::diagnostics::ErrorFrame::new("gallery.waveform.live.read"),
            )
            .into());
        }
        let len = (range.end - range.start) as usize;
        let offset = (range.start - storage.end.saturating_sub(storage.capacity as u64)) as usize;
        for (target, source) in out.channels_mut().iter_mut().zip(&storage.channels) {
            let (first, second) = source.as_slices();
            let left = offset.min(first.len());
            let first_len = len.min(first.len() - left);
            target.extend_from_slice(&first[left..left + first_len]);
            let right = offset.saturating_sub(first.len());
            target.extend_from_slice(&second[right..right + len - first_len]);
        }
        if self.3 {
            let rate = u64::from(self.1);
            for (channel, offset) in [(1, 2 * rate), (2, 4 * rate)] {
                let period = 8 * rate;
                let mut gap_start = (range.start / period) * period + offset;
                while gap_start < range.end {
                    let start = gap_start.max(range.start);
                    let end = (gap_start + rate).min(range.end);
                    if start < end {
                        out.missing_mut()[channel]
                            .push((start - range.start) as usize..(end - range.start) as usize);
                    }
                    gap_start += period;
                }
            }
        }
        Ok(WaveformReadStatus::Ready(request.clone()))
    }
}

#[derive(Debug)]
struct LiveSourcePoisoned;
impl std::fmt::Display for LiveSourcePoisoned {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("live source lock poisoned")
    }
}
impl std::error::Error for LiveSourcePoisoned {}
