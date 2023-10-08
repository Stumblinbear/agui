use std::marker::PhantomData;

use agui_core::widget::Widget;
use agui_elements::stateless::{StatelessBuildContext, StatelessWidget};
use agui_inheritance::ContextInheritedMut;
use agui_macros::StatelessWidget;
use agui_primitives::sized_box::SizedBox;

use crate::EventChannel;

#[derive(Debug, StatelessWidget)]
pub struct EventListener<Event: 'static> {
    pub phantom: PhantomData<Event>,
}

impl<Event> StatelessWidget for EventListener<Event> {
    fn build(&self, ctx: &mut StatelessBuildContext<Self>) -> Widget {
        ctx.depend_on_inherited_widget::<EventChannel<Event>>();

        SizedBox::shrink().into()
    }
}
