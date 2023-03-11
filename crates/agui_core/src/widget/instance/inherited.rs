use std::{marker::PhantomData, rc::Rc};

use crate::{
    callback::CallbackId,
    render::canvas::Canvas,
    unit::{Constraints, Data, IntrinsicDimension, Size},
    widget::{inheritance::InheritanceScope, InheritedWidget, LayoutContext, WidgetRef},
};

use super::{
    AnyWidget, ElementWidget, WidgetBuildContext, WidgetCallbackContext,
    WidgetIntrinsicSizeContext, WidgetLayoutContext,
};

pub struct InheritedInstance<W>
where
    W: AnyWidget + InheritedWidget,
{
    widget: Rc<W>,

    scope: InheritanceScope,

    child: WidgetRef,
}

impl<W> InheritedInstance<W>
where
    W: AnyWidget + InheritedWidget,
{
    pub fn new(widget: Rc<W>, child: WidgetRef) -> Self {
        Self {
            widget,

            scope: InheritanceScope::default(),

            child,
        }
    }
}

impl<W> ElementWidget for InheritedInstance<W>
where
    W: AnyWidget + InheritedWidget,
{
    fn type_name(&self) -> &'static str {
        let type_name = self.widget.type_name();

        type_name
            .split('<')
            .next()
            .unwrap_or(type_name)
            .split("::")
            .last()
            .unwrap_or(type_name)
    }

    fn is_similar(&self, other: &WidgetRef) -> bool {
        if let Some(other) = other.downcast::<W>() {
            Rc::ptr_eq(&self.widget, &other)
        } else {
            false
        }
    }

    fn get_widget(&self) -> Rc<dyn AnyWidget> {
        Rc::clone(&self.widget) as Rc<dyn AnyWidget>
    }

    fn intrinsic_size(&self, _: WidgetIntrinsicSizeContext, _: IntrinsicDimension, _: f32) -> f32 {
        0.0
    }

    fn layout(&self, _: WidgetLayoutContext, _: Constraints) -> Size {
        Size::new(0.0, 0.0)
    }

    fn build(&mut self, _: WidgetBuildContext) -> Vec<WidgetRef> {
        vec![self.child.clone()]
    }

    fn update(&mut self, old: WidgetRef) -> bool {
        let old = old
            .downcast::<W>()
            .expect("cannot update a widget instance with a different type");

        self.widget.should_notify(old.as_ref())
    }

    fn paint(&self, _: Size) -> Option<Canvas> {
        None
    }

    fn call(&mut self, _: WidgetCallbackContext, _: CallbackId, _: &Box<dyn Data>) -> bool {
        false
    }
}

impl<W> std::fmt::Debug for InheritedInstance<W>
where
    W: AnyWidget + InheritedWidget + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("InheritedInstance");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}
