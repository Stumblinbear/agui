use agui_core::{
    unit::Axis,
    widget::{BuildContext, Widget, WidgetBuild},
};
use agui_macros::{build, StatelessWidget};

use crate::intrinsic::IntrinsicAxis;

/// See [`IntrinsicAxis`] for more information.
#[derive(StatelessWidget, Debug)]
#[prop(field_defaults(default))]
pub struct IntrinsicHeight {
    #[prop(setter(into))]
    pub child: Option<Widget>,
}

impl WidgetBuild for IntrinsicHeight {
    fn build(&self, _: &mut BuildContext<Self>) -> Widget {
        build! {
            <IntrinsicAxis> {
                axis: Axis::Vertical,
                child: self.child.clone(),
            }
        }
    }
}
