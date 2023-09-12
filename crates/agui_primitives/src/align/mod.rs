use agui_core::{
    unit::{Alignment, Constraints, Size},
    widget::{BuildContext, LayoutContext, Widget, WidgetLayout},
};
use agui_macros::LayoutWidget;

mod center;

pub use center::*;

#[derive(LayoutWidget, Debug)]
pub struct Align {
    pub alignment: Alignment,

    #[prop(default, setter(strip_option))]
    pub width_factor: Option<f32>,
    #[prop(default, setter(strip_option))]
    pub height_factor: Option<f32>,

    #[prop(default, setter(into))]
    pub child: Option<Widget>,
}

impl WidgetLayout for Align {
    fn build(&self, _: &mut BuildContext<Self>) -> Vec<Widget> {
        Vec::from_iter(self.child.clone())
    }

    fn layout(&self, ctx: &mut LayoutContext, constraints: Constraints) -> Size {
        let shrink_wrap_width = self.width_factor.is_some() || !constraints.has_bounded_width();
        let shrink_wrap_height = self.height_factor.is_some() || !constraints.has_bounded_height();

        if let Some(mut child) = ctx.iter_children_mut().next() {
            let child_size = child.compute_layout(constraints.loosen());

            let size = constraints.constrain(Size {
                width: shrink_wrap_width
                    .then(|| child_size.width * self.width_factor.unwrap_or(1.0))
                    .unwrap_or(f32::INFINITY),

                height: shrink_wrap_height
                    .then(|| child_size.height * self.height_factor.unwrap_or(1.0))
                    .unwrap_or(f32::INFINITY),
            });

            child.set_offset(self.alignment.along_size(size - child_size));

            size
        } else {
            constraints.constrain(Size {
                width: if shrink_wrap_width {
                    0.0
                } else {
                    f32::INFINITY
                },

                height: if shrink_wrap_height {
                    0.0
                } else {
                    f32::INFINITY
                },
            })
        }
    }
}
