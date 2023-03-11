pub trait InheritedWidget: Sized + 'static {
    #[allow(unused_variables)]
    fn should_notify(&self, old: &Self) -> bool {
        true
    }
}
