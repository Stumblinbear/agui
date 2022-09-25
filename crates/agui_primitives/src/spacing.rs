use agui_core::{
    unit::{Layout, Sizing, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder},
};

#[derive(Debug, Default, PartialEq)]
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
    fn build(&self, _: &mut BuildContext<Self>) -> BuildResult {
        BuildResult {
            layout: Layout {
                sizing: self.0,
                ..Layout::default()
            },

            ..BuildResult::default()
        }
    }
}
