use std::ops::Deref;
use tokio::sync::broadcast::{ self, error::RecvError };

use rosrust::error::Result;
use rosrust::Message;

pub struct Subscriber<M> {
    rx: broadcast::Receiver<M>,
    // Only used in order to create new receivers
    tx: broadcast::Sender<M>,
    raii: rosrust::Subscriber,
}

impl<M> Subscriber<M> {
    fn new(rx: broadcast::Receiver<M>, tx: broadcast::Sender<M>, raii: rosrust::Subscriber) -> Subscriber<M> {
        Subscriber { rx, tx, raii }
    }
}

impl<M: Message> Subscriber<M> {
    pub async fn recv(&mut self) -> Option<M> {
        loop {
            match self.try_recv().await {
                Ok(msg) => return Some(msg),
                Err(RecvError::Closed) => return None,
                // If we lost some messages, it's fine, get the newer ones.
                Err(RecvError::Lagged(_)) => (),
            }
        }
    }

    pub async fn try_recv(&mut self) -> std::result::Result<M, RecvError> {
        self.rx.recv().await
    }
}

impl<M> Clone for Subscriber<M> {
    fn clone(&self) -> Subscriber<M> {
        // All of the inner types are cheap to clone
        Subscriber::new(self.tx.subscribe(), self.tx.clone(), self.raii.clone())
    }
}

impl<M> Deref for Subscriber<M> {
    type Target = rosrust::Subscriber;

    fn deref(&self) -> &rosrust::Subscriber {
        &self.raii
    }
}

pub fn subscribe<M: Message>(topic: impl AsRef<str>, queue_size: usize) -> Result<Subscriber<M>> {
    let (tx, rx) = broadcast::channel(1);
    let tx_clone = tx.clone();
    let raii = rosrust::subscribe(topic.as_ref(), queue_size, move |msg: M| {
        tx_clone.send(msg).unwrap();
    })?;

    Ok(Subscriber::new(rx, tx, raii))
}