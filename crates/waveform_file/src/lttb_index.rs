use crate::DiskReadStats;
use crate::recording::{
    Result, error, invalid, read_at, read_indexed_payload, u64_at, write_indexed_payload,
};
use bevy_widgetry_waveform::{
    ChannelView, LttbReducer, ReducedChannel, ReductionInput, WaveformPoint, WaveformReducer,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Seek, SeekFrom};
use std::ops::Range;

const BLOCK: u64 = 65_536;
const STRIDES: [u64; 6] = [16, 64, 256, 1024, 4096, 16_384];

#[derive(Serialize, Deserialize)]
pub(super) struct LttbLevel {
    stride: u64,
    table: u64,
}

pub(super) fn validate(
    index: &[Vec<LttbLevel>],
    channels: usize,
    frames: u64,
    raw_end: u64,
    payload_end: u64,
) -> Result<()> {
    if index.len() != channels {
        return Err(invalid());
    }
    let count = frames.div_ceil(BLOCK);
    for levels in index {
        if levels.len() != STRIDES.len() {
            return Err(invalid());
        }
        for (level, stride) in levels.iter().zip(STRIDES) {
            if level.stride != stride
                || level.table < raw_end
                || count
                    .checked_mul(16)
                    .and_then(|n| level.table.checked_add(n))
                    .is_none_or(|end| end > payload_end)
            {
                return Err(invalid());
            }
        }
    }
    Ok(())
}

pub(super) fn build(file: &mut File, frames: u64, channels: usize) -> Result<Vec<Vec<LttbLevel>>> {
    let count = frames.div_ceil(BLOCK);
    let mut index = Vec::with_capacity(channels);
    for channel in 0..channels {
        let raw = 8 + channel as u64 * frames * 5;
        let mut levels = Vec::new();
        for stride in STRIDES {
            let table = file.seek(SeekFrom::End(0)).map_err(error)?;
            file.set_len(table.checked_add(count * 16).ok_or_else(invalid)?)
                .map_err(error)?;
            for block in 0..count {
                // Each resolution starts from the original channel samples.
                // Reusing a previous LTTB level would change triangle candidates and means.
                let points = reduce_raw(
                    file,
                    raw,
                    block * BLOCK..((block + 1) * BLOCK).min(frames),
                    stride,
                    &mut DiskReadStats::default(),
                )?;
                let mut bytes = Vec::with_capacity(points.len() * 16);
                for point in points {
                    bytes.extend_from_slice(&point.sample.to_le_bytes());
                    bytes.extend_from_slice(&point.value.to_le_bytes());
                    bytes.extend_from_slice(&u32::from(point.break_before).to_le_bytes());
                }
                write_indexed_payload(file, table, block, &bytes, 16)?;
            }
            levels.push(LttbLevel { stride, table });
        }
        index.push(levels);
    }
    Ok(index)
}

fn reduce_raw(
    file: &mut File,
    raw: u64,
    range: Range<u64>,
    stride: u64,
    stats: &mut DiskReadStats,
) -> Result<Vec<WaveformPoint>> {
    if range.is_empty() {
        return Ok(Vec::new());
    }
    let start = range.start.saturating_sub(1);
    let mut bytes = vec![0; (range.end - start) as usize * 5];
    read_at(file, raw + start * 5, &mut bytes, stats)?;
    let skip = (range.start - start) as usize;
    let preceding_valid = skip == 1 && bytes[4] == 1;
    let mut values = Vec::with_capacity((range.end - range.start) as usize);
    let mut missing: Vec<Range<u64>> = Vec::new();
    let mut first_valid = false;
    for (index, cell) in bytes.as_chunks::<5>().0.iter().skip(skip).enumerate() {
        let sample = range.start + index as u64;
        let value = f32::from_le_bytes(cell[..4].try_into().map_err(error)?);
        match cell[4] {
            0 => {
                values.push(0.0);
                if let Some(last) = missing.last_mut().filter(|last| last.end == sample) {
                    last.end += 1;
                } else {
                    missing.push(sample..sample + 1);
                }
            }
            1 if value.is_finite() => {
                values.push(value);
                if index == 0 {
                    first_valid = true;
                }
            }
            _ => return Err(invalid()),
        }
    }
    let views = [ChannelView {
        first: &values,
        start: range.start,
        missing_first: &missing,
        ..Default::default()
    }];
    let mut output = Vec::new();
    LttbReducer::default().reduce(
        ReductionInput {
            channels: &views,
            buffered: range.clone(),
            viewport: range.clone(),
            appended: range.clone(),
            output_len: (range.end - range.start).div_ceil(stride) as usize,
            rebuild: true,
        },
        &mut output,
    );
    let Some(ReducedChannel::Polyline(mut points)) = output.pop() else {
        return Err(error("LTTB reducer returned invalid output"));
    };
    if preceding_valid
        && first_valid
        && let Some(first) = points.first_mut()
    {
        first.break_before = false;
    }
    Ok(points)
}

pub(super) fn query(
    file: &mut File,
    levels: &[LttbLevel],
    raw: u64,
    frames: u64,
    payload_end: u64,
    range: Range<u64>,
    output_len: usize,
    stats: &mut DiskReadStats,
) -> Result<Vec<WaveformPoint>> {
    if range.is_empty() {
        return Ok(Vec::new());
    }
    let density = (range.end - range.start).div_ceil(output_len as u64).max(1);
    let level = levels.iter().rfind(|level| level.stride <= density);
    let mut result = Vec::new();
    if let Some(level) = level {
        stats.bucket_size = level.stride;
        let blocks = range.start / BLOCK..range.end.div_ceil(BLOCK);
        let bytes = read_indexed_payload(
            file,
            level.table,
            blocks.clone(),
            16,
            BLOCK,
            payload_end,
            stats,
        )?;
        let mut previous = None;
        for cell in bytes.as_chunks::<16>().0 {
            let sample = u64_at(&cell[..8])?;
            let value = f32::from_le_bytes(cell[8..12].try_into().map_err(error)?);
            let split = u32::from_le_bytes(cell[12..16].try_into().map_err(error)?);
            if sample < blocks.start * BLOCK
                || sample >= frames.min(blocks.end * BLOCK)
                || previous.is_some_and(|p| p >= sample)
                || !value.is_finite()
                || split > 1
            {
                return Err(invalid());
            }
            previous = Some(sample);
            if range.contains(&sample) {
                result.push(WaveformPoint {
                    sample,
                    value,
                    break_before: split == 1,
                });
            }
        }
    } else {
        stats.bucket_size = density;
        let mut start = range.start;
        while start < range.end {
            let end = ((start / BLOCK + 1) * BLOCK).min(range.end);
            result.extend(reduce_raw(file, raw, start..end, density, stats)?);
            start = end;
        }
    }
    if result
        .first()
        .is_none_or(|point| point.sample != range.start)
        && let Some(point) = endpoint(file, raw, range.start, true, stats)?
    {
        result.insert(0, point);
    }
    if result
        .last()
        .is_none_or(|point| point.sample != range.end - 1)
        && let Some(point) = endpoint(file, raw, range.end - 1, false, stats)?
    {
        result.push(point);
    }
    if let Some(first) = result.first_mut() {
        first.break_before = true;
    }
    Ok(result)
}

fn endpoint(
    file: &mut File,
    raw: u64,
    sample: u64,
    break_before: bool,
    stats: &mut DiskReadStats,
) -> Result<Option<WaveformPoint>> {
    let mut cell = [0; 5];
    read_at(file, raw + sample * 5, &mut cell, stats)?;
    let value = f32::from_le_bytes(cell[..4].try_into().map_err(error)?);
    match cell[4] {
        0 => Ok(None),
        1 if value.is_finite() => Ok(Some(WaveformPoint {
            sample,
            value,
            break_before,
        })),
        _ => Err(invalid()),
    }
}
