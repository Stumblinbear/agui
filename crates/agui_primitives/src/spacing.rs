use agui_core::{
    unit::{Layout, LayoutType, Sizing, Units},
    widget::{BuildContext, Children, LayoutContext, LayoutResult, WidgetView},
};
use agui_macros::StatelessWidget;

#[derive(StatelessWidget, Debug, Default, PartialEq)]
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

impl WidgetView for Spacing {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout {
                sizing: self.0,
                ..Layout::default()
            },
        }
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::none()
    }
}
