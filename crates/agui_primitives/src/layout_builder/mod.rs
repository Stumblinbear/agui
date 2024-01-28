use std::sync::Arc;

use agui_core::{
    element::{Element, ElementBuilder},
    unit::Constraints,
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use crate::layout_builder::element::LayoutBuilderElement;

mod element;

#[derive(WidgetProps, Debug)]
pub struct LayoutBuilder<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Send + Sync + 'static,
    Param: PartialEq + Send + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
{
    #[prop(into)]
    pub resolver: Arc<ResolverFn>,
    pub builder: BuildFn,
}

impl<ResolverFn, Param, BuildFn> IntoWidget for LayoutBuilder<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Send + Sync + 'static,
    Param: PartialEq + Send + Sync + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
{
    fn into_widget(self) -> Widget {
        Widget::new(self)
    }
}

impl<ResolverFn, Param, BuildFn> ElementBuilder for LayoutBuilder<ResolverFn, Param, BuildFn>
where
    ResolverFn: Fn(Constraints) -> Param + Send + Sync + 'static,
    Param: PartialEq + Send + Sync + 'static,
    BuildFn: Fn(&Param) -> Widget + 'static,
{
    type Element = LayoutBuilderElement<ResolverFn, Param, BuildFn>;

    fn create_element(self: std::rc::Rc<Self>) -> Element
    where
        Self: Sized,
    {
        Element::new_deferred(LayoutBuilderElement::new(self))
    }
}
