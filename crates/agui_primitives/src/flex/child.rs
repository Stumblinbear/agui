use agui_core::widget::Widget;

use super::{FlexFit, Flexible};

#[derive(Debug, Clone)]
pub struct FlexChild {
    pub flex: f32,
    pub fit: FlexFit,

    pub child: Widget,
}

impl From<Widget> for FlexChild {
    fn from(widget: Widget) -> Self {
        if let Some(flexible) = widget.downcast::<Flexible>() {
            FlexChild {
                flex: flexible.flex.unwrap_or(0.0),
                fit: flexible.fit.unwrap_or_default(),

                child: widget,
            }
        } else {
            FlexChild {
                flex: 0.0,
                fit: FlexFit::default(),

                child: widget,
            }
        }
    }
}
