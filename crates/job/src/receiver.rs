use std::sync::Arc;

use crate::channel::Data;
use crate::{
    manager::{Shared, lock},
    *,
};
use bevy_widgetry_diagnostics::ErrorFrame;

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

    pub async fn send(&self, item: T) -> Result<SendStatus, SendFailure> {
        let sent = match futures_util::future::select(
            Box::pin(self.cancelled()),
            self.data.sender.broadcast(Arc::new(item)),
        )
        .await
        {
            futures_util::future::Either::Left(_) => return Ok(SendStatus::Canceled),
            futures_util::future::Either::Right((sent, _)) => sent,
        };
        match sent {
            Ok(None) => {
                self.shared.signal.event.notify(usize::MAX);
                self.shared.signal.wake.notify();
                Ok(SendStatus::Sent)
            }
            Ok(Some(_)) => Err(SendFailure::new(
                SendError::Overflowed { job: self.id() },
                ErrorFrame::new("job.send").field("job_id", self.id().get()),
            )),
            Err(_) if !self.data.sender.is_closed() => Ok(SendStatus::Sent),
            Err(_) => Err(SendFailure::new(
                SendError::Closed { job: self.id() },
                ErrorFrame::new("job.send").field("job_id", self.id().get()),
            )),
        }
    }

    pub fn send_blocking(&self, item: T) -> Result<SendStatus, SendFailure> {
        bevy::tasks::block_on(self.send(item))
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

    pub fn try_recv(&mut self) -> Result<JobRead<T>, ReceiveFailure> {
        if self.finished {
            return Ok(JobRead::Closed);
        }
        let id = lock(&self.shared.lifecycle).status.id;
        self.manager.reap(id);
        let received = self.data.try_recv();
        self.map_received(received)
    }

    fn map_received(
        &mut self,
        received: Result<Arc<T>, async_broadcast::TryRecvError>,
    ) -> Result<JobRead<T>, ReceiveFailure> {
        match received {
            Ok(item) => {
                if !self.data.is_empty() || lock(&self.shared.lifecycle).status.outcome.is_some() {
                    self.shared.signal.wake.notify();
                }
                return Ok(JobRead::Item(item));
            }
            Err(async_broadcast::TryRecvError::Overflowed(count)) => {
                let job = lock(&self.shared.lifecycle).status.id;
                return Err(ReceiveFailure::new(
                    ReceiveError::Overflowed { job, count },
                    ErrorFrame::new("job.try_recv")
                        .field("job_id", job.get())
                        .field("count", count),
                ));
            }
            Err(async_broadcast::TryRecvError::Empty) => return Ok(JobRead::Empty),
            Err(async_broadcast::TryRecvError::Closed) => {}
        }
        let Some(outcome) = lock(&self.shared.lifecycle).status.outcome.clone() else {
            return Ok(JobRead::Empty);
        };
        self.finished = true;
        Ok(JobRead::Finished(outcome))
    }

    pub async fn recv(&mut self) -> Result<JobRead<T>, ReceiveFailure> {
        loop {
            let listener = self.shared.signal.event.listen();
            match self.try_recv() {
                Ok(JobRead::Empty) => listener.await,
                result => return result,
            }
        }
    }

    pub fn recv_blocking(&mut self) -> Result<JobRead<T>, ReceiveFailure> {
        bevy::tasks::block_on(self.recv())
    }
}

#[cfg(test)]
#[allow(clippy::disallowed_macros)]
mod tests {
    const TEST_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

    use super::*;
    use bevy::tasks::{IoTaskPool, TaskPool};
    use std::time::Instant;

    #[test]
    fn stale_empty_observation_cannot_skip_the_last_item() {
        IoTaskPool::get_or_init(TaskPool::new);
        let manager = JobManager::default();
        let release = event_listener::Event::new();
        let listener = release.listen();
        let (id, mut receiver) = manager
            .spawn_io::<usize, _, _>(JobOptions::default(), move |context| async move {
                listener.await;
                assert_eq!(context.send(42).await?, SendStatus::Sent);
                Ok(())
            })
            .unwrap();
        let stale = receiver.data.try_recv();
        assert_eq!(stale, Err(async_broadcast::TryRecvError::Empty));
        release.notify(usize::MAX);
        let deadline = Instant::now() + TEST_WAIT_TIMEOUT;
        while !manager.status(id).unwrap().state.is_finished() {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(matches!(
            receiver.map_received(stale).unwrap(),
            JobRead::Empty
        ));
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Item(value)) if *value == 42));
        assert!(matches!(
            receiver.try_recv(),
            Ok(JobRead::Finished(JobOutcome::Completed))
        ));
        assert!(matches!(receiver.try_recv(), Ok(JobRead::Closed)));
    }
}
