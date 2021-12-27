use downcast_rs::{impl_downcast, Downcast};

pub trait Value: Downcast + Send + Sync + 'static {}

impl<T> Value for T where T: Send + Sync + 'static {}

impl_downcast!(Value);
