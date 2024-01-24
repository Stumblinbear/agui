mod unbounded;

pub use unbounded::{unbounded, UnboundedReceiver, UnboundedSender};

#[derive(Debug, thiserror::Error)]
pub enum SendError<T> {
    #[error("no receivers")]
    NoReceivers(T),

    #[error("channel closed")]
    Closed(T),
}
