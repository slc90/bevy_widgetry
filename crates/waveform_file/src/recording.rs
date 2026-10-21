use bevy_widgetry_waveform::{
    RangeBoundary, ReducedChannel, WaveformAlgorithm, WaveformMetadata, WaveformPoint,
    WaveformReadError, WaveformSessionKind, WaveformSpan, WaveformUnit,
};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::Path;

const MAGIC: &[u8; 8] = b"WFRIDX02";
const BASE: u64 = 256;
const MAX_BUCKET: u64 = 65_536;
pub(super) type Result<T> = std::result::Result<T, WaveformReadError>;

#[derive(Clone, Serialize, Deserialize)]
struct Level {
    size: u64,
    table: u64,
    count: u64,
}

#[derive(Serialize, Deserialize)]
struct Header {
    rate: u32,
    frames: u64,
    unit: u8,
    labels: Vec<String>,
    levels: Vec<Vec<Level>>,
    lttb: Vec<Vec<crate::lttb_index::LttbLevel>>,
}

pub struct Recording {
    file: File,
    header: Header,
    metadata: WaveformMetadata,
    payload_end: u64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DiskReadStats {
    pub bytes_read: u64,
    pub peak_buffer_bytes: usize,
    pub bucket_size: u64,
}

pub struct DiskQuery {
    pub channels: Vec<ReducedChannel>,
    pub stats: DiskReadStats,
}

pub(super) fn error(error: impl std::fmt::Display) -> WaveformReadError {
    WaveformReadError(error.to_string())
}

pub(super) fn invalid() -> WaveformReadError {
    error("invalid or truncated waveform recording")
}

pub(super) fn read_at(
    file: &mut File,
    offset: u64,
    bytes: &mut [u8],
    stats: &mut DiskReadStats,
) -> Result<()> {
    file.seek(SeekFrom::Start(offset)).map_err(error)?;
    file.read_exact(bytes).map_err(error)?;
    stats.bytes_read += bytes.len() as u64;
    stats.peak_buffer_bytes = stats.peak_buffer_bytes.max(bytes.len());
    Ok(())
}

pub(super) fn u64_at(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_le_bytes(bytes.try_into().map_err(error)?))
}

fn append_span(spans: &mut Vec<WaveformSpan>, span: WaveformSpan) {
    if let Some(last) = spans
        .last_mut()
        .filter(|last| last.samples.end == span.samples.start)
    {
        last.samples.end = span.samples.end;
        last.min = last.min.min(span.min);
        last.max = last.max.max(span.max);
    } else {
        spans.push(span);
    }
}

fn raw_spans(
    file: &mut File,
    raw: u64,
    range: Range<u64>,
    bucket_size: u64,
    stats: &mut DiskReadStats,
) -> Result<Vec<WaveformSpan>> {
    let mut bytes = vec![0; (range.end - range.start) as usize * 5];
    read_at(file, raw + range.start * 5, &mut bytes, stats)?;
    let mut spans = Vec::new();
    for (index, cell) in bytes.as_chunks::<5>().0.iter().enumerate() {
        let value = f32::from_le_bytes(cell[..4].try_into().map_err(error)?);
        match cell[4] {
            0 => {}
            1 if value.is_finite() => {
                let span = WaveformSpan {
                    samples: RangeBoundary {
                        start: range.start + index as u64,
                        end: range.start + index as u64 + 1,
                    },
                    min: value,
                    max: value,
                };
                if spans.last().is_some_and(|last: &WaveformSpan| {
                    last.samples.start / bucket_size == span.samples.start / bucket_size
                }) {
                    append_span(&mut spans, span);
                } else {
                    spans.push(span);
                }
            }
            _ => return Err(invalid()),
        }
    }
    Ok(spans)
}

fn read_bucket(
    file: &mut File,
    level: &Level,
    index: u64,
    payload_end: u64,
    stats: &mut DiskReadStats,
) -> Result<Vec<WaveformSpan>> {
    read_buckets(file, level, index..index + 1, payload_end, stats)
}

fn read_buckets(
    file: &mut File,
    level: &Level,
    buckets: Range<u64>,
    payload_end: u64,
    stats: &mut DiskReadStats,
) -> Result<Vec<WaveformSpan>> {
    if buckets.start >= buckets.end || buckets.end > level.count {
        return Err(invalid());
    }
    let bytes = read_indexed_payload(
        file,
        level.table,
        buckets.clone(),
        24,
        level.size,
        payload_end,
        stats,
    )?;
    let mut spans = Vec::with_capacity(bytes.len() / 24);
    let mut previous = buckets.start * level.size;
    for cell in bytes.as_chunks::<24>().0 {
        let start = u64_at(&cell[..8])?;
        let end = u64_at(&cell[8..16])?;
        let min = f32::from_le_bytes(cell[16..20].try_into().map_err(error)?);
        let max = f32::from_le_bytes(cell[20..24].try_into().map_err(error)?);
        if start < previous
            || start >= end
            || end > buckets.end * level.size
            || !min.is_finite()
            || !max.is_finite()
            || min > max
        {
            return Err(invalid());
        }
        previous = end;
        spans.push(WaveformSpan {
            samples: RangeBoundary { start, end },
            min,
            max,
        });
    }
    Ok(spans)
}

pub(super) fn read_indexed_payload(
    file: &mut File,
    table: u64,
    buckets: Range<u64>,
    record_size: u64,
    max_count: u64,
    payload_end: u64,
    stats: &mut DiskReadStats,
) -> Result<Vec<u8>> {
    let length = (buckets.end - buckets.start)
        .checked_mul(16)
        .ok_or_else(invalid)?;
    let mut entries = vec![0; usize::try_from(length).map_err(error)?];
    read_at(file, table + buckets.start * 16, &mut entries, stats)?;
    let offset = u64_at(&entries[..8])?;
    let mut end = offset;
    for entry in entries.as_chunks::<16>().0 {
        let start = u64_at(&entry[..8])?;
        let count = u64_at(&entry[8..])?;
        if start != end || count > max_count {
            return Err(invalid());
        }
        end = start
            .checked_add(count.checked_mul(record_size).ok_or_else(invalid)?)
            .ok_or_else(invalid)?;
        if end > payload_end {
            return Err(invalid());
        }
    }
    let length = end - offset;
    let mut bytes = vec![0; usize::try_from(length).map_err(error)?];
    read_at(file, offset, &mut bytes, stats)?;
    Ok(bytes)
}

fn write_bucket(file: &mut File, level: &Level, index: u64, spans: &[WaveformSpan]) -> Result<()> {
    let mut bytes = Vec::with_capacity(spans.len() * 24);
    for span in spans {
        bytes.extend_from_slice(&span.samples.start.to_le_bytes());
        bytes.extend_from_slice(&span.samples.end.to_le_bytes());
        bytes.extend_from_slice(&span.min.to_le_bytes());
        bytes.extend_from_slice(&span.max.to_le_bytes());
    }
    write_indexed_payload(file, level.table, index, &bytes, 24)
}

pub(super) fn write_indexed_payload(
    file: &mut File,
    table: u64,
    index: u64,
    bytes: &[u8],
    record_size: usize,
) -> Result<()> {
    let offset = file.seek(SeekFrom::End(0)).map_err(error)?;
    file.write_all(bytes).map_err(error)?;
    file.seek(SeekFrom::Start(table + index * 16))
        .map_err(error)?;
    file.write_all(&offset.to_le_bytes()).map_err(error)?;
    file.write_all(&((bytes.len() / record_size) as u64).to_le_bytes())
        .map_err(error)?;
    Ok(())
}

/// The producer fills each channel chunk with samples in the declared unit or None for gaps.
/// The destination is published only after all raw samples and index levels have been written.
pub fn build_recording(
    path: &Path,
    metadata: WaveformMetadata,
    mut produce: impl FnMut(usize, Range<u64>, &mut Vec<Option<f32>>) -> Result<()>,
) -> Result<()> {
    metadata.validate().map_err(error)?;
    let WaveformSessionKind::Replay {
        total_frames: frames,
    } = metadata.kind
    else {
        return Err(error("disk recording requires Replay metadata"));
    };
    let raw_end = frames
        .checked_mul(metadata.channel_count as u64)
        .and_then(|n| n.checked_mul(5))
        .and_then(|n| n.checked_add(8))
        .ok_or_else(invalid)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(error)?;
    let file = temp.as_file_mut();
    file.write_all(MAGIC).map_err(error)?;
    let mut values = Vec::new();
    let mut bytes = Vec::new();
    for channel in 0..metadata.channel_count {
        let mut start = 0;
        while start < frames {
            let end = (start + 65_536).min(frames);
            values.clear();
            produce(channel, start..end, &mut values)?;
            if values.len() as u64 != end - start || values.iter().flatten().any(|v| !v.is_finite())
            {
                return Err(error("producer must supply complete finite chunks"));
            }
            bytes.clear();
            for value in &values {
                bytes.extend_from_slice(&value.unwrap_or(0.0).to_le_bytes());
                bytes.push(u8::from(value.is_some()));
            }
            file.write_all(&bytes).map_err(error)?;
            start = end;
        }
    }
    let mut levels = Vec::new();
    for channel in 0..metadata.channel_count {
        let mut channel_levels: Vec<Level> = Vec::new();
        let mut size = BASE;
        loop {
            let count = frames.div_ceil(size);
            let table = file.seek(SeekFrom::End(0)).map_err(error)?;
            let table_end = count
                .checked_mul(16)
                .and_then(|n| n.checked_add(table))
                .ok_or_else(invalid)?;
            file.set_len(table_end).map_err(error)?;
            let level = Level { size, table, count };
            for index in 0..count {
                let mut stats = DiskReadStats::default();
                let spans = if let Some(lower) = channel_levels.last() {
                    let mut merged = Vec::new();
                    for child in index * 2..(index * 2 + 2).min(lower.count) {
                        let end = file.metadata().map_err(error)?.len();
                        for span in read_bucket(file, lower, child, end, &mut stats)? {
                            append_span(&mut merged, span);
                        }
                    }
                    merged
                } else {
                    raw_spans(
                        file,
                        8 + channel as u64 * frames * 5,
                        index * size..((index + 1) * size).min(frames),
                        size,
                        &mut stats,
                    )?
                };
                write_bucket(file, &level, index, &spans)?;
            }
            channel_levels.push(level);
            if count == 1 || size == MAX_BUCKET {
                break;
            }
            size = size.checked_mul(2).ok_or_else(invalid)?;
        }
        levels.push(channel_levels);
    }
    if file.metadata().map_err(error)?.len() < raw_end {
        return Err(invalid());
    }
    let lttb = crate::lttb_index::build(file, frames, metadata.channel_count)?;
    let header = Header {
        rate: metadata.sample_rate,
        frames,
        unit: match metadata.unit {
            WaveformUnit::Volts => 0,
            WaveformUnit::Millivolts => 1,
            WaveformUnit::Microvolts => 2,
        },
        labels: metadata.channel_labels,
        levels,
        lttb,
    };
    let json = serde_json::to_vec(&header).map_err(error)?;
    file.seek(SeekFrom::End(0)).map_err(error)?;
    file.write_all(&json).map_err(error)?;
    file.write_all(&(json.len() as u64).to_le_bytes())
        .map_err(error)?;
    file.write_all(MAGIC).map_err(error)?;
    file.sync_all().map_err(error)?;
    temp.persist_noclobber(path).map_err(error)?;
    Ok(())
}

impl Recording {
    pub fn open(path: &Path) -> Result<Self> {
        let mut file = File::open(path).map_err(error)?;
        let length = file.metadata().map_err(error)?.len();
        if length < 24 {
            return Err(invalid());
        }
        let mut stats = DiskReadStats::default();
        let mut magic = [0; 8];
        read_at(&mut file, 0, &mut magic, &mut stats)?;
        if &magic != MAGIC {
            return Err(invalid());
        }
        let mut footer = [0; 16];
        read_at(&mut file, length - 16, &mut footer, &mut stats)?;
        let size = u64_at(&footer[..8])?;
        if &footer[8..] != MAGIC || size > 8_388_608 || size > length - 24 {
            return Err(invalid());
        }
        let payload_end = length - 16 - size;
        let mut json = vec![0; size as usize];
        read_at(&mut file, payload_end, &mut json, &mut stats)?;
        let header: Header = serde_json::from_slice(&json).map_err(error)?;
        let unit = match header.unit {
            0 => WaveformUnit::Volts,
            1 => WaveformUnit::Millivolts,
            2 => WaveformUnit::Microvolts,
            _ => return Err(invalid()),
        };
        let metadata = WaveformMetadata {
            sample_rate: header.rate,
            channel_count: header.labels.len(),
            channel_labels: header.labels.clone(),
            unit,
            kind: WaveformSessionKind::Replay {
                total_frames: header.frames,
            },
        };
        metadata.validate().map_err(error)?;
        let raw_end = header
            .frames
            .checked_mul(metadata.channel_count as u64)
            .and_then(|n| n.checked_mul(5))
            .and_then(|n| n.checked_add(8))
            .ok_or_else(invalid)?;
        if raw_end > payload_end || header.levels.len() != metadata.channel_count {
            return Err(invalid());
        }
        for levels in &header.levels {
            let mut size = BASE;
            if levels.is_empty() {
                return Err(invalid());
            }
            for level in levels {
                if level.size != size
                    || level.count != header.frames.div_ceil(size)
                    || level.table < raw_end
                    || level
                        .count
                        .checked_mul(16)
                        .and_then(|n| n.checked_add(level.table))
                        .is_none_or(|end| end > payload_end)
                {
                    return Err(invalid());
                }
                size = size.checked_mul(2).ok_or_else(invalid)?;
            }
            if levels
                .last()
                .is_none_or(|l| l.count != 1 && l.size != MAX_BUCKET)
            {
                return Err(invalid());
            }
        }
        crate::lttb_index::validate(
            &header.lttb,
            metadata.channel_count,
            header.frames,
            raw_end,
            payload_end,
        )?;
        Ok(Self {
            file,
            header,
            metadata,
            payload_end,
        })
    }

    pub fn metadata(&self) -> &WaveformMetadata {
        &self.metadata
    }

    pub fn query(
        &mut self,
        range: Range<u64>,
        output_len: usize,
        algorithm: WaveformAlgorithm,
    ) -> Result<DiskQuery> {
        self.query_channels(range, output_len, algorithm, 0..self.metadata.channel_count)
    }

    pub fn query_channels(
        &mut self,
        range: Range<u64>,
        output_len: usize,
        algorithm: WaveformAlgorithm,
        channel_range: Range<usize>,
    ) -> Result<DiskQuery> {
        if range.start > range.end || range.end > self.header.frames || output_len == 0 {
            return Err(error("invalid disk query range or density"));
        }
        if channel_range.start > channel_range.end
            || channel_range.end > self.metadata.channel_count
        {
            return Err(error("invalid disk query channel range"));
        }
        let mut stats = DiskReadStats::default();
        let mut channels = Vec::with_capacity(channel_range.len());
        for channel in channel_range {
            let raw = 8 + channel as u64 * self.header.frames * 5;
            if algorithm == WaveformAlgorithm::Lttb {
                let points = crate::lttb_index::query(
                    &mut self.file,
                    &self.header.lttb[channel],
                    raw,
                    self.header.frames,
                    self.payload_end,
                    range.clone(),
                    output_len,
                    &mut stats,
                )?;
                channels.push(ReducedChannel::Polyline(points));
                continue;
            }
            let levels = &self.header.levels[channel];
            let density = (range.end - range.start).div_ceil(output_len as u64).max(1);
            let level = levels.iter().rposition(|level| level.size <= density);
            stats.bucket_size = level.map_or(1, |i| levels[i].size);
            if range.end - range.start <= output_len as u64 {
                let mut bytes = vec![0; (range.end - range.start) as usize * 5];
                read_at(
                    &mut self.file,
                    raw + range.start * 5,
                    &mut bytes,
                    &mut stats,
                )?;
                let mut points = Vec::new();
                let mut gap = true;
                for (index, cell) in bytes.as_chunks::<5>().0.iter().enumerate() {
                    let value = f32::from_le_bytes(cell[..4].try_into().map_err(error)?);
                    match cell[4] {
                        0 => gap = true,
                        1 if value.is_finite() => {
                            points.push(WaveformPoint {
                                sample: range.start + index as u64,
                                value,
                                break_before: gap,
                            });
                            gap = false;
                        }
                        _ => return Err(invalid()),
                    }
                }
                channels.push(ReducedChannel::Polyline(points));
            } else {
                let mut spans = Vec::new();
                query_spans(
                    &mut self.file,
                    levels,
                    level,
                    raw,
                    self.header.frames,
                    self.payload_end,
                    range.clone(),
                    density.min(BASE),
                    &mut spans,
                    &mut stats,
                )?;
                channels.push(ReducedChannel::Envelope(spans));
            }
        }
        stats.peak_buffer_bytes += channels
            .iter()
            .map(|c| match c {
                ReducedChannel::Envelope(v) => v.capacity() * size_of::<WaveformSpan>(),
                ReducedChannel::Polyline(v) => v.capacity() * size_of::<WaveformPoint>(),
            })
            .sum::<usize>();
        Ok(DiskQuery { channels, stats })
    }
}

fn query_spans(
    file: &mut File,
    levels: &[Level],
    level: Option<usize>,
    raw: u64,
    frames: u64,
    payload_end: u64,
    range: Range<u64>,
    raw_bucket_size: u64,
    output: &mut Vec<WaveformSpan>,
    stats: &mut DiskReadStats,
) -> Result<()> {
    if range.is_empty() {
        return Ok(());
    }
    let Some(i) = level else {
        // Only boundary fragments or sub-base-density buckets reach raw storage.
        // Keep each read bounded even for a long window at a narrow pixel density.
        let mut start = range.start;
        while start < range.end {
            let end = (start + MAX_BUCKET).min(range.end);
            output.extend(raw_spans(file, raw, start..end, raw_bucket_size, stats)?);
            start = end;
        }
        return Ok(());
    };
    let current = &levels[i];
    let first_full = range.start.div_ceil(current.size);
    let last_full = if range.end == frames {
        range.end.div_ceil(current.size)
    } else {
        range.end / current.size
    };
    let first = range.start / current.size;
    let last = range.end.div_ceil(current.size);
    if first < first_full {
        let end = ((first + 1) * current.size).min(range.end);
        query_spans(
            file,
            levels,
            i.checked_sub(1),
            raw,
            frames,
            payload_end,
            range.start..end,
            raw_bucket_size,
            output,
            stats,
        )?;
    }
    if first_full < last_full {
        output.extend(read_buckets(
            file,
            current,
            first_full..last_full,
            payload_end,
            stats,
        )?);
    }
    if last_full < last && last_full >= first_full {
        let bucket = last_full;
        let start = bucket * current.size;
        let end = ((bucket + 1) * current.size).min(frames);
        query_spans(
            file,
            levels,
            i.checked_sub(1),
            raw,
            frames,
            payload_end,
            range.start.max(start)..range.end.min(end),
            raw_bucket_size,
            output,
            stats,
        )?;
    }
    Ok(())
}
