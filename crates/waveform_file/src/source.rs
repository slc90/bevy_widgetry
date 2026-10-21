use crate::Recording;
use bevy_widgetry_waveform::{
    PlanarBuffer, WaveformMetadata, WaveformReadError, WaveformReadRequest, WaveformReadStatus,
    WaveformReducedReadStatus, WaveformSource,
};
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};

type Reply = Result<Vec<bevy_widgetry_waveform::ReducedChannel>, WaveformReadError>;

struct Slot {
    request: WaveformReadRequest,
    result: Option<Reply>,
}

#[derive(Default)]
struct State {
    slots: BTreeMap<u64, Slot>,
    queued: VecDeque<u64>,
    shutdown: bool,
}

pub struct DiskWaveformSource {
    metadata: WaveformMetadata,
    state: Arc<(Mutex<State>, Condvar)>,
}

impl DiskWaveformSource {
    /// Opening validates metadata synchronously, so applications should load recordings off the UI thread.
    pub fn open(path: &Path) -> Result<Self, WaveformReadError> {
        let mut recording = Recording::open(path)?;
        let metadata = recording.metadata().clone();
        let state = Arc::new((Mutex::new(State::default()), Condvar::new()));
        let shared = state.clone();
        std::thread::Builder::new()
            .name("waveform disk reader".into())
            .spawn(move || {
                loop {
                    let (lock, wake) = &*shared;
                    let Ok(mut state) = lock.lock() else {
                        return;
                    };
                    while state.queued.is_empty() && !state.shutdown {
                        let Ok(next) = wake.wait(state) else {
                            return;
                        };
                        state = next;
                    }
                    if state.shutdown {
                        return;
                    }
                    let Some(reader) = state.queued.pop_front() else {
                        continue;
                    };
                    let Some(slot) = state.slots.get(&reader) else {
                        continue;
                    };
                    let request = slot.request.clone();
                    drop(state);
                    let result = recording
                        .query(request.range.clone(), request.output_len, request.algorithm)
                        .map(|result| result.channels);
                    let Ok(mut state) = lock.lock() else {
                        return;
                    };
                    if let Some(slot) = state
                        .slots
                        .get_mut(&reader)
                        .filter(|slot| slot.request == request)
                    {
                        slot.result = Some(result);
                    }
                }
            })
            .map_err(|e| WaveformReadError(format!("cannot start disk reader: {e}")))?;
        Ok(Self { metadata, state })
    }
}

impl WaveformSource for DiskWaveformSource {
    fn metadata(&self) -> Result<WaveformMetadata, WaveformReadError> {
        Ok(self.metadata.clone())
    }
    fn provides_reduced(&self) -> bool {
        true
    }
    fn read(
        &self,
        _: &WaveformReadRequest,
        _: &mut PlanarBuffer,
    ) -> Result<WaveformReadStatus, WaveformReadError> {
        Err(WaveformReadError(
            "disk source requires reduced reads".into(),
        ))
    }
    fn read_reduced(
        &self,
        request: &WaveformReadRequest,
    ) -> Result<WaveformReducedReadStatus, WaveformReadError> {
        let (lock, wake) = &*self.state;
        let mut state = lock
            .lock()
            .map_err(|e| WaveformReadError(format!("disk reader lock poisoned: {e}")))?;
        if let Some(slot) = state
            .slots
            .get_mut(&request.reader_id)
            .filter(|slot| slot.request == *request)
        {
            if let Some(result) = slot.result.take() {
                state.slots.remove(&request.reader_id);
                return result.map(|channels| WaveformReducedReadStatus::Ready {
                    request: request.clone(),
                    channels,
                });
            }
        } else {
            state.slots.insert(
                request.reader_id,
                Slot {
                    request: request.clone(),
                    result: None,
                },
            );
            if !state.queued.contains(&request.reader_id) {
                state.queued.push_back(request.reader_id);
            }
            wake.notify_one();
        }
        Ok(WaveformReducedReadStatus::Pending)
    }
    fn cancel_reads(&self, reader_id: u64) {
        let mut state = self
            .state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.slots.remove(&reader_id);
        state.queued.retain(|reader| *reader != reader_id);
    }
}

impl Drop for DiskWaveformSource {
    fn drop(&mut self) {
        let mut state = self
            .state
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.shutdown = true;
        self.state.1.notify_one();
    }
}
