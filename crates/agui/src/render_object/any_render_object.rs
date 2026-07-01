use std::{any::Any, cell::RefCell, rc::Rc};

use crate::{
    diagnostics::{Diagnostics, DiagnosticsNode},
    render_object::RenderObject,
};

pub trait AnyRenderObject {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn render_object_name(&self) -> &str;

    fn dyn_describe(&self, d: &mut Diagnostics) -> DiagnosticsNode;
}

impl<T> AnyRenderObject for T
where
    T: Any,
    T: RenderObject,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn render_object_name(&self) -> &str {
        std::any::type_name::<T>()
    }

    fn dyn_describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.describe(d)
    }
}

impl<T> RenderObject for Box<T>
where
    T: AnyRenderObject + ?Sized + 'static,
{
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        (**self).dyn_describe(d)
    }
}

impl<T> RenderObject for Rc<RefCell<T>>
where
    T: AnyRenderObject + ?Sized + 'static,
{
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        self.borrow().dyn_describe(d)
    }
}
