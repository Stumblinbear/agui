use std::error::Error;

pub trait BindRenderer<T> {
    #[allow(async_fn_in_trait)]
    async fn bind(self, target: &T) -> Result<Box<dyn Renderer>, Box<dyn Error + Send + Sync>>
    where
        Self: Sized;
}

pub trait Renderer {
    fn render_notifier(&self) -> async_channel::Receiver<()>;

    fn render(&mut self);
}
