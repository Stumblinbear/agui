use agui_core::{
    unit::Alignment,
    widget::{BuildContext, Children, WidgetRef, WidgetView},
};
use agui_macros::StatelessWidget;

use crate::Align;

#[derive(StatelessWidget, Debug, Default)]
pub struct Center {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,

    pub child: WidgetRef,
}

impl WidgetView for Center {
    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::new(
            Align {
                alignment: Alignment::CENTER,

                width_factor: self.width_factor,
                height_factor: self.height_factor,

                child: self.child.clone(),
            }
            .into(),
        )
    }
}
