use std::any::Any;

pub trait AsAny: Any {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str;

    fn short_type_name(&self) -> &'static str;
}

impl<T> AsAny for T
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

    fn short_type_name(&self) -> &'static str {
        let type_name = std::any::type_name::<T>();

        type_name
            .split('<')
            .next()
            .unwrap_or(type_name)
            .split("::")
            .last()
            .unwrap_or(type_name)
    }
}

impl dyn AsAny {
    pub fn downcast_ref<T: AsAny>(&self) -> Option<&T> {
        AsAny::as_any(self).downcast_ref()
    }

    pub fn downcast_mut<T: AsAny>(&mut self) -> Option<&mut T> {
        AsAny::as_any_mut(self).downcast_mut()
    }
}
