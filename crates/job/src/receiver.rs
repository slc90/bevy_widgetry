use std::sync::Arc;

use crate::channel::Data;
use crate::{
    manager::{Shared, lock},
    *,
};
use bevy_widgetry_internal_log::widgetry_error;

pub struct JobContext<T> {
    shared: Arc<Shared>,
    data: Arc<Data<T>>,
}

impl<T: Send + Sync + 'static> JobContext<T> {
    pub(crate) fn new(shared: Arc<Shared>, data: Arc<Data<T>>) -> Self {
        Self { shared, data }
    }

    pub fn id(&self) -> JobId {
        lock(&self.shared.lifecycle).status.id
    }

    pub fn is_cancelled(&self) -> bool {
        lock(&self.shared.lifecycle).cancel_requested
    }

    pub fn acknowledge_cancel(&self) -> bool {
        lock(&self.shared.lifecycle).acknowledge()
    }

    pub async fn cancelled(&self) {
        loop {
            let listener = self.shared.cancel.listen();
            if self.is_cancelled() {
                return;
            }
            listener.await;
        }
    }

    pub async fn send(&self, item: T) -> Result<(), SendError> {
        let sent = match futures_util::future::select(
            Box::pin(self.cancelled()),
            self.data.sender.broadcast(Arc::new(item)),
        )
        .await
        {
            futures_util::future::Either::Left(_) => return Err(SendError::Canceled),
            futures_util::future::Either::Right((sent, _)) => sent,
        };
        match sent {
            Ok(None) => {
                self.shared.signal.0.notify(usize::MAX);
                Ok(())
            }
            Ok(Some(_)) => {
                widgetry_error!(job_id = self.id().get(), "Job 广播意外覆盖未读数据");
                Err(SendError::Overflowed)
            }
            Err(_) if !self.data.sender.is_closed() => Ok(()),
            Err(_) => Err(SendError::Closed),
        }
    }
}

pub struct JobReceiver<T> {
    manager: JobManager,
    shared: Arc<Shared>,
    finished: bool,
    data: async_broadcast::Receiver<Arc<T>>,
}

impl<T> JobReceiver<T> {
    pub(crate) fn new(
        manager: JobManager,
        shared: Arc<Shared>,
        data: async_broadcast::Receiver<Arc<T>>,
    ) -> Self {
        Self {
            manager,
            shared,
            finished: false,
            data,
        }
    }

    pub fn try_recv(&mut self) -> Result<JobMessage<T>, TryRecvError> {
        if self.finished {
            return Err(TryRecvError::Closed);
        }
        let id = lock(&self.shared.lifecycle).status.id;
        self.manager.reap(id);
        let received = self.data.try_recv();
        self.map_received(received)
    }

    fn map_received(
        &mut self,
        received: Result<Arc<T>, async_broadcast::TryRecvError>,
    ) -> Result<JobMessage<T>, TryRecvError> {
        match received {
            Ok(item) => return Ok(JobMessage::Item(item)),
            Err(async_broadcast::TryRecvError::Overflowed(count)) => {
                widgetry_error!(
                    job_id = lock(&self.shared.lifecycle).status.id.get(),
                    count,
                    "Job 广播违反不丢失结果契约"
                );
                return Err(TryRecvError::Overflowed(count));
            }
            Err(async_broadcast::TryRecvError::Empty) => return Err(TryRecvError::Empty),
            Err(async_broadcast::TryRecvError::Closed) => {}
        }
        let Some(outcome) = lock(&self.shared.lifecycle).status.outcome.clone() else {
            return Err(TryRecvError::Empty);
        };
        self.finished = true;
        Ok(JobMessage::Finished(outcome))
    }

    pub async fn recv(&mut self) -> Result<JobMessage<T>, TryRecvError> {
        loop {
            let listener = self.shared.signal.0.listen();
            match self.try_recv() {
                Err(TryRecvError::Empty) => listener.await,
                result => return result,
            }
        }
    }

    pub fn recv_blocking(&mut self) -> Result<JobMessage<T>, TryRecvError> {
        bevy::tasks::block_on(self.recv())
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    use super::*;
    use bevy::tasks::{IoTaskPool, TaskPool};
    use std::time::{Duration, Instant};

    #[test]
    fn stale_empty_observation_cannot_skip_the_last_item() {
        IoTaskPool::get_or_init(TaskPool::new);
        let manager = JobManager::default();
        let release = event_listener::Event::new();
        let listener = release.listen();
        let (id, mut receiver) = manager
            .spawn_io::<usize, _, _, SendError>(JobOptions::default(), move |context| async move {
                listener.await;
                context.send(42).await?;
                Ok(())
            })
            .unwrap();
        let stale = receiver.data.try_recv();
        assert_eq!(stale, Err(async_broadcast::TryRecvError::Empty));
        release.notify(usize::MAX);
        let deadline = Instant::now() + Duration::from_secs(5);
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(
            receiver.map_received(stale).unwrap_err(),
            TryRecvError::Empty
        );
        assert!(matches!(receiver.try_recv(), Ok(JobMessage::Item(value)) if *value == 42));
        assert!(matches!(
            receiver.try_recv(),
            Ok(JobMessage::Finished(JobOutcome::Completed))
        ));
        assert_eq!(receiver.try_recv().unwrap_err(), TryRecvError::Closed);
    }
}
