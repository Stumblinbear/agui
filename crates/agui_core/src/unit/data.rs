use downcast_rs::{impl_downcast, Downcast};

pub trait Data: Downcast {}

impl<T> Data for T where T: Downcast {}

impl_downcast!(Data);
