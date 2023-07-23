use std::marker::PhantomData;

use agui_core::widget::{BuildContext, ContextWidgetMut, WidgetBuild};
use agui_macros::StatelessWidget;

use crate::EventChannel;

#[derive(Debug, StatelessWidget)]
pub struct EventListener<Event: 'static> {
    pub phantom: PhantomData<Event>,
}

impl<Event> WidgetBuild for EventListener<Event> {
    type Child = ();

    fn build(&self, ctx: &mut BuildContext<Self>) -> Self::Child {
        ctx.depend_on_inherited_widget::<EventChannel<Event>>();
    }
}
