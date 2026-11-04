use crate::{RecordingError, RecordingFailure};
use bevy_widgetry_diagnostics::ErrorFrame;
use bevy_widgetry_waveform::{WaveformAlgorithm, WaveformReadError};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
    ops::Range,
    path::Path,
};

pub(super) struct FileAccess<'a> {
    file: &'a mut File,
    path: &'a Path,
    position: Option<u64>,
    pub channel: Option<usize>,
    pub algorithm: Option<WaveformAlgorithm>,
    pub range: Option<Range<u64>>,
    pub output_len: Option<usize>,
    pub channels: Option<Range<usize>>,
    pub temp_path: Option<&'a Path>,
}

impl<'a> FileAccess<'a> {
    pub fn new(file: &'a mut File, path: &'a Path) -> Self {
        Self {
            file,
            path,
            position: None,
            channel: None,
            algorithm: None,
            range: None,
            output_len: None,
            channels: None,
            temp_path: None,
        }
    }

    pub fn frame(&self, operation: &'static str) -> ErrorFrame {
        let mut frame = ErrorFrame::new(operation).field("path", self.path.display());
        if let Some(path) = self.temp_path {
            frame = frame.field("temp_path", path.display());
        }
        if let Some(channel) = self.channel {
            frame = frame.field("channel", channel);
        }
        if let Some(algorithm) = self.algorithm {
            frame = frame.field("algorithm", format_args!("{algorithm:?}"));
        }
        if let Some(range) = &self.range {
            frame = frame.field("range", format_args!("{range:?}"));
        }
        if let Some(output_len) = self.output_len {
            frame = frame.field("output_len", output_len);
        }
        if let Some(channels) = &self.channels {
            frame = frame.field("channel_range", format_args!("{channels:?}"));
        }
        frame
    }

    #[track_caller]
    pub fn fail(&self, kind: RecordingError, operation: &'static str) -> WaveformReadError {
        RecordingFailure::new(kind, self.frame(operation)).into()
    }

    #[track_caller]
    fn io_error(
        &self,
        source: std::io::Error,
        operation: &'static str,
        offset: Option<u64>,
        length: Option<u64>,
    ) -> WaveformReadError {
        self.fail(
            RecordingError::Io {
                source,
                offset,
                length,
            },
            operation,
        )
    }

    pub fn seek(&mut self, position: SeekFrom) -> Result<u64, WaveformReadError> {
        let offset = match position {
            SeekFrom::Start(offset) => Some(offset),
            _ => None,
        };
        let result = self
            .file
            .seek(position)
            .map_err(|source| self.io_error(source, "waveform_file.seek", offset, None))?;
        self.position = Some(result);
        Ok(result)
    }

    pub fn write_all(&mut self, bytes: &[u8]) -> Result<(), WaveformReadError> {
        self.file.write_all(bytes).map_err(|source| {
            self.io_error(
                source,
                "waveform_file.write",
                self.position,
                Some(bytes.len() as u64),
            )
        })?;
        self.position = self
            .position
            .and_then(|p| p.checked_add(bytes.len() as u64));
        Ok(())
    }

    pub fn read_exact(&mut self, bytes: &mut [u8]) -> Result<(), WaveformReadError> {
        self.file.read_exact(bytes).map_err(|source| {
            self.io_error(
                source,
                "waveform_file.read",
                self.position,
                Some(bytes.len() as u64),
            )
        })?;
        self.position = self
            .position
            .and_then(|p| p.checked_add(bytes.len() as u64));
        Ok(())
    }

    pub fn set_len(&self, length: u64) -> Result<(), WaveformReadError> {
        self.file
            .set_len(length)
            .map_err(|source| self.io_error(source, "waveform_file.set_len", None, Some(length)))
    }

    pub fn metadata(&self) -> Result<std::fs::Metadata, WaveformReadError> {
        self.file
            .metadata()
            .map_err(|source| self.io_error(source, "waveform_file.metadata", None, None))
    }

    pub fn sync_all(&self) -> Result<(), WaveformReadError> {
        self.file
            .sync_all()
            .map_err(|source| self.io_error(source, "waveform_file.sync", None, None))
    }

    pub fn decode(
        &self,
        source: std::array::TryFromSliceError,
        length: usize,
    ) -> WaveformReadError {
        self.fail(
            RecordingError::Decode { source, length },
            "waveform_file.decode",
        )
    }

    pub fn usize(&self, value: u64) -> Result<usize, WaveformReadError> {
        usize::try_from(value).map_err(|source| {
            self.fail(
                RecordingError::Conversion { source, value },
                "waveform_file.length",
            )
        })
    }

    pub fn overflow(&self, operation: &'static str, left: u64, right: u64) -> WaveformReadError {
        self.fail(
            RecordingError::Overflow {
                operation,
                left,
                right,
            },
            "waveform_file.layout",
        )
    }
}

#[track_caller]
pub(super) fn failure(
    path: &Path,
    error: RecordingError,
    operation: &'static str,
) -> WaveformReadError {
    RecordingFailure::new(
        error,
        ErrorFrame::new(operation).field("path", path.display()),
    )
    .into()
}
