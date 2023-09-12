use std::marker::PhantomData;

use agui_core::widget::{BuildContext, ContextInheritedMut, Widget, WidgetBuild};
use agui_macros::StatelessWidget;
use agui_primitives::sized_box::SizedBox;

use crate::EventChannel;

#[derive(Debug, StatelessWidget)]
pub struct EventListener<Event: 'static> {
    pub phantom: PhantomData<Event>,
}

impl<Event> WidgetBuild for EventListener<Event> {
    fn build(&self, ctx: &mut BuildContext<Self>) -> Widget {
        ctx.depend_on_inherited_widget::<EventChannel<Event>>();

        SizedBox::shrink().into()
    }
}
