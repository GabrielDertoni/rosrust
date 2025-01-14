use std::ops::Deref;
use tokio::task;

use rosrust::error::Result as RosResult;
use rosrust::Message;

#[derive(Clone)]
pub struct Publisher<M: Message> {
    inner: rosrust::Publisher<M>,
}

impl<M: Message> Publisher<M> {
    pub fn new(topic: impl AsRef<str>, queue_size: usize) -> RosResult<Publisher<M>> {
        let inner = rosrust::publish(topic.as_ref(), queue_size)?;
        Ok(Publisher{ inner })
    }

    #[inline]
    pub fn set_latching(&mut self, latching: bool) {
        self.inner.set_latching(latching);
    }

    #[inline]
    pub fn set_queue_size(&mut self, queue_size: usize) {
        self.inner.set_queue_size(queue_size);
    }

    // I don't think this future is cancellable as is.
    // NOTE: Don't use in select.
    pub async fn send(&mut self, message: M) -> RosResult<()> {
        let self_clone = self.clone();
        let handle = task::spawn_blocking(move || self_clone.inner.send(message));
        handle.await.unwrap()
    }
}

impl<M: Message> Deref for Publisher<M> {
    type Target = rosrust::Publisher<M>;

    fn deref(&self) -> &rosrust::Publisher<M> {
        &self.inner
    }
}