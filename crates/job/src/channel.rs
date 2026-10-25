use std::{any::Any, sync::Arc};

pub(crate) trait ResultChannel: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn close(&self);
}

pub(crate) struct Data<T> {
    pub sender: async_broadcast::Sender<Arc<T>>,
    _inactive: async_broadcast::InactiveReceiver<Arc<T>>,
}

impl<T: Send + Sync + 'static> Data<T> {
    pub fn new(capacity: usize) -> (Arc<Self>, async_broadcast::Receiver<Arc<T>>) {
        let (mut sender, receiver) = async_broadcast::broadcast(capacity);
        sender.set_overflow(false);
        sender.set_await_active(false);
        let inactive = sender.new_receiver().deactivate();
        (
            Arc::new(Self {
                sender,
                _inactive: inactive,
            }),
            receiver,
        )
    }
}

impl<T: Send + Sync + 'static> ResultChannel for Data<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn close(&self) {
        self.sender.close();
    }
}
