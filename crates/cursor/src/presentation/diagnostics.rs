use bevy::{
    asset::{AssetLoadError, AssetPath, LoadState},
    prelude::*,
    render::render_resource::TextureFormat,
};
use bevy_widgetry_diagnostics::{
    ErrorFrame, WidgetryFailure,
    bevy::{FailureState, WidgetryEcsError},
};
use std::{error::Error, fmt, sync::Arc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Pixels {
    asset: AssetId<Image>,
    expected: UVec2,
    actual: UVec2,
    format: TextureFormat,
    bytes: Option<usize>,
}

pub(super) enum FailureKey {
    Load {
        asset: AssetId<Image>,
        source: Arc<AssetLoadError>,
    },
    Pixels(Pixels),
}

impl PartialEq for FailureKey {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::Load {
                    asset: a,
                    source: x,
                },
                Self::Load {
                    asset: b,
                    source: y,
                },
            ) => a == b && Arc::ptr_eq(x, y),
            (Self::Pixels(a), Self::Pixels(b)) => a == b,
            _ => false,
        }
    }
}
impl Eq for FailureKey {}

#[derive(Debug)]
pub(super) enum CursorError {
    Load {
        asset: AssetId<Image>,
        path: Option<Box<AssetPath<'static>>>,
        source: Arc<AssetLoadError>,
    },
    Pixels {
        pixels: Pixels,
        path: Option<Box<AssetPath<'static>>>,
    },
}

impl fmt::Display for CursorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load { asset, path, .. } => {
                write!(f, "Cursor asset failed: {asset:?} ({path:?})")
            }
            Self::Pixels { pixels, path } => write!(
                f,
                "Cursor requires expected RGBA dimensions and CPU pixels: {pixels:?} ({path:?})"
            ),
        }
    }
}
impl Error for CursorError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Load { source, .. } => Some(source.as_ref()),
            Self::Pixels { .. } => None,
        }
    }
}

fn frame(handle: &Handle<Image>) -> ErrorFrame {
    ErrorFrame::new("cursor.image")
        .field("asset", format_args!("{:?}", handle.id()))
        .field("path", format_args!("{:?}", handle.path()))
}

pub(super) fn usable(
    server: &AssetServer,
    images: &Assets<Image>,
    handle: &Handle<Image>,
    expected: UVec2,
    state: &mut FailureState<FailureKey>,
) -> bool {
    if let Some(LoadState::Failed(source)) = server.get_load_state(handle.id()) {
        state.failure_batch(
            FailureKey::Load {
                asset: handle.id(),
                source: source.clone(),
            },
            || load_failure(handle, source),
        );
        return false;
    }
    let Some(image) = images.get(handle) else {
        return false;
    };
    let pixels = Pixels {
        asset: handle.id(),
        expected,
        actual: image.size(),
        format: image.texture_descriptor.format,
        bytes: image.data.as_ref().map(Vec::len),
    };
    let rgba8_bytes_per_pixel = 4;
    let rgba32_float_bytes_per_pixel = 16;
    let bytes_per_pixel = match pixels.format {
        TextureFormat::Rgba8Unorm | TextureFormat::Rgba8UnormSrgb => Some(rgba8_bytes_per_pixel),
        TextureFormat::Rgba32Float => Some(rgba32_float_bytes_per_pixel),
        _ => None,
    };
    let expected_bytes = bytes_per_pixel.and_then(|bytes| {
        u64::from(expected.x)
            .checked_mul(u64::from(expected.y))
            .and_then(|area| area.checked_mul(bytes))
            .and_then(|bytes| usize::try_from(bytes).ok())
    });
    if pixels.actual != expected || expected_bytes.is_none() || pixels.bytes != expected_bytes {
        state.failure(FailureKey::Pixels(pixels), || {
            Arc::new(WidgetryFailure::new(
                CursorError::Pixels {
                    pixels,
                    path: handle.path().cloned().map(Box::new),
                },
                frame(handle)
                    .field("expected", expected)
                    .field("actual", pixels.actual)
                    .field("format", format_args!("{:?}", pixels.format))
                    .field("bytes", format_args!("{:?}", pixels.bytes)),
            ))
        });
        return false;
    }
    state.recover(|| frame(handle));
    true
}

fn load_failure(handle: &Handle<Image>, source: Arc<AssetLoadError>) -> WidgetryEcsError {
    if let AssetLoadError::AssetLoaderError(loader) = source.as_ref()
        && let Some(managed) = loader.error().downcast_ref::<WidgetryEcsError>()
    {
        return managed.clone().with_context(frame(handle));
    }
    WidgetryEcsError::new(WidgetryFailure::new(
        CursorError::Load {
            asset: handle.id(),
            path: handle.path().cloned().map(Box::new),
            source,
        },
        frame(handle),
    ))
}

pub(super) fn handled(state: &FailureState<FailureKey>, window: Entity, outcome: &str) {
    if state.active().is_some() {
        let outcome = format!("window={window:?}; {outcome}");
        for diagnostic in state.diagnostics() {
            diagnostic.record_handled("选择可用的光标回退", &outcome);
        }
    }
}

#[cfg(test)]
mod tests;
