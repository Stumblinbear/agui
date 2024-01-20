use std::sync::Arc;

use futures::{channel::mpsc, lock::Mutex, StreamExt};

#[derive(Clone)]
pub struct UpdateNotifier {
    notifier: Arc<Notifier>,
}

struct Notifier {
    sender: mpsc::Sender<()>,
    receiver: Mutex<mpsc::Receiver<()>>,
}

impl UpdateNotifier {
    pub(crate) fn new() -> Self {
        let (sender, receiver) = mpsc::channel(1);

        Self {
            notifier: Arc::new(Notifier {
                sender,
                receiver: Mutex::new(receiver),
            }),
        }
    }

    pub fn notify(&self) {
        let _ = self.notifier.sender.clone().try_send(());
    }

    pub async fn wait(&self) {
        self.notifier.receiver.lock().await.next().await;
    }
}
