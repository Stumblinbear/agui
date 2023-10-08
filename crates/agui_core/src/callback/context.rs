use super::CallbackQueue;

pub trait ContextCallbackQueue {
    fn get_callback_queue(&self) -> &CallbackQueue;
}
