use agui_core::{
    unit::Alignment,
    widget::{BuildContext, IntoWidget, Widget, WidgetBuild},
};
use agui_macros::StatelessWidget;

use crate::align::Align;

#[derive(Debug, StatelessWidget)]
#[prop(field_defaults(default))]
pub struct Center {
    #[prop(setter(strip_option))]
    pub width_factor: Option<f32>,
    #[prop(setter(strip_option))]
    pub height_factor: Option<f32>,

    #[prop(setter(into))]
    pub child: Option<Widget>,
}

impl WidgetBuild for Center {
    fn build(&self, _: &mut BuildContext<Self>) -> Widget {
        Align {
            alignment: Alignment::CENTER,

            width_factor: self.width_factor,
            height_factor: self.height_factor,

            child: self.child.clone(),
        }
        .into_widget()
    }
}
