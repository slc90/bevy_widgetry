use std::{marker::PhantomData, sync::Arc};

use crate::{
    manager::{Shared, lock},
    *,
};

pub struct JobContext<T> {
    shared: Arc<Shared>,
    marker: PhantomData<T>,
}

impl<T> JobContext<T> {
    pub(crate) fn new(shared: Arc<Shared>) -> Self {
        Self {
            shared,
            marker: PhantomData,
        }
    }

    pub fn id(&self) -> JobId {
        lock(&self.shared.status).id
    }
}

pub struct JobReceiver<T> {
    manager: JobManager,
    shared: Arc<Shared>,
    finished: bool,
    marker: PhantomData<T>,
}

impl<T> JobReceiver<T> {
    pub(crate) fn new(manager: JobManager, shared: Arc<Shared>) -> Self {
        Self {
            manager,
            shared,
            finished: false,
            marker: PhantomData,
        }
    }

    pub fn try_recv(&mut self) -> Result<JobMessage<T>, TryRecvError> {
        if self.finished {
            return Err(TryRecvError::Closed);
        }
        let id = lock(&self.shared.status).id;
        self.manager.reap(id);
        let Some(outcome) = lock(&self.shared.status).outcome.clone() else {
            return Err(TryRecvError::Empty);
        };
        self.finished = true;
        Ok(JobMessage::Finished(outcome))
    }

    pub async fn recv(&mut self) -> Option<JobMessage<T>> {
        loop {
            let listener = self.shared.signal.0.listen();
            match self.try_recv() {
                Ok(message) => return Some(message),
                Err(TryRecvError::Closed) => return None,
                Err(TryRecvError::Empty) => listener.await,
            }
        }
    }
}
