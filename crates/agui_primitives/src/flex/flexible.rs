use agui_core::widget::{BuildContext, Widget, WidgetBuild};
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

impl WidgetBuild for Flexible {
    fn build(&self, _: &mut BuildContext<Self>) -> Widget {
        self.child.clone()
    }
}
