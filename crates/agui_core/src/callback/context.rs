use super::CallbackQueue;

pub trait ContextCallbackQueue {
    fn callback_queue(&self) -> &CallbackQueue;
}
