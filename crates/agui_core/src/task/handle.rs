use futures::prelude::future::RemoteHandle;

pub struct TaskHandle<T> {
    inner: RemoteHandle<T>,
}

impl<T> From<RemoteHandle<T>> for TaskHandle<T> {
    fn from(inner: RemoteHandle<T>) -> Self {
        Self { inner }
    }
}
