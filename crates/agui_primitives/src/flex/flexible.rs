use agui_core::widget::Widget;
use agui_elements::stateless::{StatelessBuildContext, StatelessWidget};
use agui_macros::StatelessWidget;

use super::FlexFit;

#[derive(Debug, Clone, StatelessWidget)]
#[props(default)]
pub struct Flexible {
    pub flex: Option<f32>,
    pub fit: Option<FlexFit>,

    #[prop(!default)]
    pub child: Widget,
}

impl StatelessWidget for Flexible {
    fn build(&self, _: &mut StatelessBuildContext<Self>) -> Widget {
        self.child.clone()
    }
}
