//! Gallery 自有文件格式与只读 adapter；测试资产只由手动工具预生成。

use bevy::asset::{AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use bevy_widgetry::waveform::{PlanarBuffer, WaveformReadError, WaveformSource};
use std::collections::VecDeque;
use std::io;
use std::ops::Range;
use std::sync::{Arc, RwLock};

/// 固定文件的 planar 数据；范围解释仍属于 WaveformConfig。
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
    type Error = io::Error;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _: &(),
        _: &mut LoadContext<'_>,
    ) -> io::Result<ReplayAsset> {
        let result = async {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            parse(&bytes).map(|data| ReplayAsset(Arc::new(data)))
        }
        .await;
        if let Err(ref failure) = result {
            error!(%failure, "Waveform replay asset 加载失败");
        }
        result
    }

    fn extensions(&self) -> &[&str] {
        &["wfrm"]
    }
}

/// 先检查版本与精确 payload 大小，拒绝截断、溢出和非有限 sample。
fn parse(bytes: &[u8]) -> io::Result<ReplayData> {
    let invalid = || io::Error::new(io::ErrorKind::InvalidData, "invalid WFRM payload");
    if bytes.len() < 24 || &bytes[..4] != b"WFRM" {
        return Err(invalid());
    }
    let word = |offset| -> io::Result<u32> {
        Ok(u32::from_le_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| invalid())?,
        ))
    };
    let rate = word(8)?;
    let count = word(12)? as usize;
    let frames = u64::from_le_bytes(bytes[16..24].try_into().map_err(|_| invalid())?);
    let length = usize::try_from(frames).map_err(|_| invalid())?;
    if word(4)? != 1
        || rate == 0
        || count == 0
        || length == 0
        || count
            .checked_mul(length)
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| n.checked_add(24))
            != Some(bytes.len())
    {
        return Err(invalid());
    }
    let mut channels = Vec::with_capacity(count);
    for channel in bytes[24..].chunks_exact(length * 4) {
        let mut values = Vec::with_capacity(length);
        for sample in channel.as_chunks::<4>().0 {
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

/// loader 完成后共享同一份 asset 内存；read 只复制请求区间。
#[derive(Default)]
pub(crate) struct ReplaySource(pub RwLock<Option<Arc<ReplayData>>>);

impl WaveformSource for ReplaySource {
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError> {
        let guard = self
            .0
            .read()
            .map_err(|_| WaveformReadError("replay source lock poisoned".into()))?;
        let data = guard
            .as_ref()
            .ok_or_else(|| WaveformReadError("replay asset not ready".into()))?;
        if range.end > data.frames
            || range.start > range.end
            || out.channels().len() != data.channels.len()
        {
            return Err(WaveformReadError("replay range outside asset".into()));
        }
        for (target, source) in out.channels_mut().iter_mut().zip(&data.channels) {
            target.extend_from_slice(&source[range.start as usize..range.end as usize]);
        }
        Ok(())
    }
}

/// producer 专属 raw ring，与 Widget 自身 ring 分离；所有 channel 共用 frame index。
pub(crate) struct LiveStorage {
    pub channels: Vec<VecDeque<f32>>,
    pub end: u64,
    pub capacity: usize,
}

pub(crate) struct LiveSource(pub RwLock<LiveStorage>);

impl LiveSource {
    pub fn new(channels: usize, capacity: usize) -> Self {
        Self(RwLock::new(LiveStorage {
            channels: (0..channels)
                .map(|_| VecDeque::with_capacity(capacity))
                .collect(),
            end: 0,
            capacity,
        }))
    }

    /// 不跳过 catch-up frame；Stress 使用便宜周期序列，Basic 使用四种肉眼可读信号。
    pub fn produce(&self, target: u64, rate: u32, basic: bool) -> Result {
        let mut storage = self
            .0
            .write()
            .map_err(|_| BevyError::error("live producer lock poisoned"))?;
        let start = storage.end;
        let capacity = storage.capacity;
        for (channel, values) in storage.channels.iter_mut().enumerate() {
            let period = u64::from(rate) * (channel as u64 % 7 + 1);
            for frame in start..target {
                let phase =
                    ((frame + period * (channel as u64 % 4) / 4) % period) as f32 / period as f32;
                let value = if basic {
                    match channel {
                        0 => (phase * std::f32::consts::TAU).sin(),
                        1 => {
                            if phase < 0.5 {
                                0.8
                            } else {
                                -0.8
                            }
                        }
                        2 => phase * 2.0 - 1.0,
                        _ => (1.0 - (phase * 2.0 - 1.0).abs() * 2.0) * 0.6,
                    }
                } else {
                    (phase * 2.0 - 1.0) * (1.0 - (channel % 4) as f32 * 0.15)
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
    fn read(&self, range: Range<u64>, out: &mut PlanarBuffer) -> Result<(), WaveformReadError> {
        let storage = self
            .0
            .read()
            .map_err(|_| WaveformReadError("live source lock poisoned".into()))?;
        if range.start > range.end
            || range.start < storage.end.saturating_sub(storage.capacity as u64)
            || range.end > storage.end
            || out.channels().len() != storage.channels.len()
        {
            return Err(WaveformReadError("live range outside source ring".into()));
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
        Ok(())
    }
}
