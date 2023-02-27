use std::any::Any;

pub trait Data: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str;
}

impl<T> Data for T
where
    T: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

impl dyn Data {
    pub fn downcast_ref<T: Data>(&self) -> Option<&T> {
        Data::as_any(self).downcast_ref()
    }

    pub fn downcast_mut<T: Data>(&mut self) -> Option<&mut T> {
        Data::as_any_mut(self).downcast_mut()
    }
}
