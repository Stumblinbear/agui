use std::sync::Arc;

use event_listener::Event;

#[derive(Clone)]
pub struct Flag {
    event: Arc<Event>,
}

impl Default for Flag {
    fn default() -> Self {
        Self {
            event: Arc::new(Event::new()),
        }
    }
}

impl Flag {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notify(&self) {
        self.event.notify(usize::MAX);
    }

    pub fn subscribe(&self) -> Subscriber {
        Subscriber {
            event: self.event.clone(),
        }
    }
}

pub struct Subscriber {
    event: Arc<Event>,
}

impl Subscriber {
    pub async fn wait(&mut self) {
        self.event.listen().await
    }
}
