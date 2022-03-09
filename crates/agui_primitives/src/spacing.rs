use agui_core::{
    unit::{Layout, Sizing, Units},
    widget::{BuildResult, WidgetBuilder, BuildContext},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Spacing(pub Sizing);

impl Spacing {
    pub fn none() -> Self {
        Self(Sizing::All(Units::Pixels(0.0)))
    }

    pub fn horizontal(units: Units) -> Self {
        Self(Sizing::Axis {
            width: units,
            height: Units::default(),
        })
    }

    pub fn vertical(units: Units) -> Self {
        Self(Sizing::Axis {
            width: Units::default(),
            height: units,
        })
    }
}

impl WidgetBuilder for Spacing {
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        ctx.set_layout(
            Layout {
                sizing: self.0,
                ..Layout::default()
            }
        );

        BuildResult::None
    }
}
