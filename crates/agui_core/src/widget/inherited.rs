pub trait InheritedWidget: Sized + 'static {
    #[allow(unused_variables)]
    fn should_notify(&self, old_widget: &Self) -> bool {
        true
    }
}
