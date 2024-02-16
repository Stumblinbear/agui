use std::rc::Rc;

use agui_core::{
    element::{deferred::ElementDeferred, lifecycle::ElementLifecycle, ElementComparison},
    unit::Constraints,
    util::ptr_eq::PtrEqual,
    widget::Widget,
};

use crate::layout_builder::LayoutBuilder;

pub struct LayoutBuilderElement<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
{
    pub(crate) widget: Rc<LayoutBuilder<ResolverFn, Param, BuilderFn>>,
}

impl<ResolverFn, Param, BuilderFn> LayoutBuilderElement<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
{
    pub fn new(widget: Rc<LayoutBuilder<ResolverFn, Param, BuilderFn>>) -> Self {
        Self { widget }
    }
}

impl<ResolverFn, Param, BuilderFn> ElementLifecycle
    for LayoutBuilderElement<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
{
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        if new_widget.is_exact_ptr(&self.widget) {
            return ElementComparison::Identical;
        }

        if let Some(new_widget) =
            new_widget.downcast::<LayoutBuilder<ResolverFn, Param, BuilderFn>>()
        {
            self.widget = new_widget;

            ElementComparison::Changed
        } else {
            ElementComparison::Invalid
        }
    }
}

impl<ResolverFn, Param, BuilderFn> ElementDeferred
    for LayoutBuilderElement<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
{
    type Param = Param;

    fn create_resolver(&self) -> impl Fn(Constraints) -> Self::Param + Send + 'static {
        self.widget.resolver.clone()
    }

    fn build(&self, param: &Self::Param) -> Widget {
        (self.widget.builder)(param)
    }
}

impl<ResolverFn, Param, BuilderFn> std::fmt::Debug
    for LayoutBuilderElement<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
    LayoutBuilder<ResolverFn, Param, BuilderFn>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("LayoutBuilderElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}
