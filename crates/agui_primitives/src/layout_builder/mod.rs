use agui_core::{
    element::{Element, ElementBuilder},
    unit::Constraints,
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use crate::layout_builder::element::LayoutBuilderElement;

mod element;

#[derive(WidgetProps, Debug)]
pub struct LayoutBuilder<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
{
    pub resolver: ResolverFn,
    pub builder: BuilderFn,
}

impl<ResolverFn, Param, BuilderFn> IntoWidget for LayoutBuilder<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
{
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl<ResolverFn, Param, BuilderFn> ElementBuilder for LayoutBuilder<ResolverFn, Param, BuilderFn>
where
    ResolverFn: Fn(Constraints) -> Param + Clone + Send + 'static,
    Param: PartialEq + Send + 'static,
    BuilderFn: Fn(&Param) -> Widget + 'static,
{
    type Element = LayoutBuilderElement<ResolverFn, Param, BuilderFn>;

    fn create_element(self: std::rc::Rc<Self>) -> Element
    where
        Self: Sized,
    {
        Element::new_deferred(LayoutBuilderElement::new(self))
    }
}
