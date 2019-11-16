use std::sync::Arc;

use crossbeam::queue::{ArrayQueue, PushError};
use crossbeam::utils::Backoff;

pub struct Sender<T> {
    queue: Arc<ArrayQueue<T>>
}

pub struct Receiver<T> {
    queue: Arc<ArrayQueue<T>>
}

impl<T> Sender<T> {
    pub fn send(&self, item: T) {
        let backoff = Backoff::new();

        let mut item = item;
        loop {
            match self.queue.push(item) {
                Ok(()) => {return}
                Err(PushError(left)) => {
                    item = left
                }
            }

            backoff.snooze();
        }
    }
}

impl<T> Receiver<T> {
    pub fn recv(&self) -> T {
        let backoff = Backoff::new();

        loop {
            match self.queue.pop() {
                Ok(item) => {
                    return item;
                }
                Err(_) => {
                    backoff.snooze();
                }
            }
        }
    }
}

pub fn bounded<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    let queue = Arc::new(ArrayQueue::new(size));

    return (Sender { queue: queue.clone() }, Receiver { queue: queue.clone()})
}