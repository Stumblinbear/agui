use std::rc::Rc;

use agui_core::{
    element::{deferred::ElementDeferred, lifecycle::ElementLifecycle, ElementComparison},
    unit::Constraints,
    util::ptr_eq::PtrEqual,
    widget::Widget,
};

use crate::layout_builder::LayoutBuilder;

pub struct LayoutBuilderElement<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
{
    pub(crate) widget: Rc<LayoutBuilder<ResolverFn, Param, BuildFn>>,
}

impl<ResolverFn, Param, BuildFn> LayoutBuilderElement<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
{
    pub fn new(widget: Rc<LayoutBuilder<ResolverFn, Param, BuildFn>>) -> Self {
        Self { widget }
    }
}

impl<ResolverFn, Param, BuildFn> ElementLifecycle
    for LayoutBuilderElement<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
{
    fn update(&mut self, new_widget: &Widget) -> ElementComparison {
        if new_widget.is_exact_ptr(&self.widget) {
            return ElementComparison::Identical;
        }

        if let Some(new_widget) = new_widget.downcast::<LayoutBuilder<ResolverFn, Param, BuildFn>>()
        {
            self.widget = new_widget;

            ElementComparison::Changed
        } else {
            ElementComparison::Invalid
        }
    }
}

impl<ResolverFn, Param, BuildFn> ElementDeferred
    for LayoutBuilderElement<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
{
    type Param = Param;

    fn create_resolver(&self) -> impl Fn(Constraints) -> Self::Param + Send + 'static {
        self.widget.resolver.clone()
    }

    fn build(&self, param: &Self::Param) -> Widget {
        (self.widget.builder)(param)
    }
}

impl<ResolverFn, Param, BuildFn> std::fmt::Debug
    for LayoutBuilderElement<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
    LayoutBuilder<ResolverFn, Param, BuildFn>: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("LayoutBuilderElement");

        dbg.field("widget", &self.widget);

        dbg.finish()
    }
}
