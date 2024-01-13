pub trait RenderWindow {
    fn render_notifier(&self) -> async_channel::Receiver<()>;

    fn render(&mut self);
}
