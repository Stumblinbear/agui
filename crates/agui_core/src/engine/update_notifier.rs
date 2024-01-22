use std::sync::Arc;

use futures::{channel::mpsc, StreamExt};
use parking_lot::Mutex;

#[derive(Clone)]
pub struct UpdateNotifier {
    // TODO: figure out a better method of notifying
    sender: Arc<Mutex<mpsc::Sender<()>>>,
}

impl UpdateNotifier {
    pub(crate) fn new() -> (Self, UpdateReceiver) {
        let (sender, receiver) = mpsc::channel(1);

        (
            Self {
                sender: Arc::new(Mutex::new(sender)),
            },
            UpdateReceiver { receiver },
        )
    }

    pub fn notify(&self) {
        let _ = self.sender.lock().try_send(());
    }
}

pub struct UpdateReceiver {
    receiver: mpsc::Receiver<()>,
}

impl UpdateReceiver {
    pub async fn wait(&mut self) {
        self.receiver.next().await;
    }
}
