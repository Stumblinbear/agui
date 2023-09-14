use agui_core::{
    unit::Alignment,
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use crate::align::Align;

#[derive(Debug, WidgetProps)]
#[props(default)]
pub struct Center {
    pub width_factor: Option<f32>,
    pub height_factor: Option<f32>,

    #[prop(into)]
    pub child: Option<Widget>,
}

impl IntoWidget for Center {
    fn into_widget(self) -> Widget {
        Align {
            alignment: Alignment::CENTER,

            width_factor: self.width_factor,
            height_factor: self.height_factor,

            child: self.child.clone(),
        }
        .into_widget()
    }
}
