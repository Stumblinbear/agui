use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use async_lock::RwLock;

use crate::broadcast::SendError;

pub use async_channel::Receiver as UnboundedReceiver;

pub fn unbounded<T>() -> (UnboundedSender<T>, async_channel::Receiver<T>)
where
    T: Clone + Send + Sync + 'static,
{
    let (tx, rx) = async_channel::unbounded();

    (
        UnboundedSender {
            channels: Arc::new(RwLock::new(vec![tx])),

            closed: Arc::new(AtomicBool::new(false)),
        },
        rx,
    )
}

// TODO: rewrite this to actually be its own, optimized implementation
pub struct UnboundedSender<T> {
    channels: Arc<RwLock<Vec<async_channel::Sender<T>>>>,

    closed: Arc<AtomicBool>,
}

impl<T> Clone for UnboundedSender<T> {
    fn clone(&self) -> Self {
        Self {
            channels: self.channels.clone(),

            closed: self.closed.clone(),
        }
    }
}

impl<T: 'static + Clone + Send + Sync> UnboundedSender<T> {
    // TODO: make this non-async
    pub async fn subscribe(&self) -> async_channel::Receiver<T> {
        let (tx, rx) = async_channel::unbounded();

        let mut channels = self.channels.write().await;

        channels.push(tx);

        rx
    }

    pub async fn send(&mut self, message: T) -> Result<(), SendError<T>> {
        let is_closed = self.closed.load(Ordering::SeqCst);

        if is_closed {
            return Err(SendError::Closed(message));
        }

        let mut channels = self.channels.write().await;

        if channels.is_empty() {
            return Err(SendError::NoReceivers(message));
        }

        let results = futures_util::future::join_all(
            channels
                .iter_mut()
                .map(|channel| channel.send(message.clone())),
        )
        .await;

        for i in results
            .into_iter()
            .enumerate()
            .filter(|(_, result)| result.is_err())
            .map(|(i, _)| i)
            .rev()
        {
            channels.remove(i);
        }

        Ok(())
    }

    /// Returns `true` if the channel is closed.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Closes the channel.
    ///
    /// Returns `true` if this call closed the channel.
    pub fn close(&self) -> bool {
        let was_closed = self.closed.swap(true, Ordering::SeqCst);

        if !was_closed {
            for channel in futures_lite::future::block_on(self.channels.write()).drain(..) {
                channel.close();
            }
        }

        !was_closed
    }
}
