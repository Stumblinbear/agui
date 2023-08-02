use agui_core::{
    unit::Alignment,
    widget::{BuildContext, Widget, WidgetBuild},
};
use agui_macros::{build, StatelessWidget};

use crate::align::Align;

#[derive(StatelessWidget, Debug, Default)]
pub struct Center {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,

    pub child: Option<Widget>,
}

impl WidgetBuild for Center {
    type Child = Widget;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        build! {
            Align {
                alignment: Alignment::CENTER,

                width_factor: self.width_factor,
                height_factor: self.height_factor,

                child: self.child.clone(),
            }
        }
    }
}
