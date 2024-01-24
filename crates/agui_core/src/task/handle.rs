use std::{
    future::{Future, IntoFuture},
    pin::Pin,
};

use futures_util::future::RemoteHandle;

pub struct TaskHandle<T> {
    inner: RemoteHandle<T>,
}

impl<T: 'static> IntoFuture for TaskHandle<T> {
    type Output = T;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(self.inner)
    }
}

impl<T> From<RemoteHandle<T>> for TaskHandle<T> {
    fn from(inner: RemoteHandle<T>) -> Self {
        Self { inner }
    }
}
