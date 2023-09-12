use agui_core::{
    unit::Alignment,
    widget::{IntoWidget, Widget},
};
use agui_macros::WidgetProps;

use crate::align::Align;

#[derive(Debug, WidgetProps)]
#[prop(field_defaults(default))]
pub struct Center {
    #[prop(setter(strip_option))]
    pub width_factor: Option<f32>,
    #[prop(setter(strip_option))]
    pub height_factor: Option<f32>,

    #[prop(setter(into))]
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
