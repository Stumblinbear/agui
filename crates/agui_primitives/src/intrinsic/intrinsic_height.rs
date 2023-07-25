use agui_core::{
    unit::Axis,
    widget::{BuildContext, WidgetBuild, WidgetRef},
};
use agui_macros::{build, StatelessWidget};

use crate::IntrinsicAxis;

/// See [`IntrinsicAxis`] for more information.
#[derive(StatelessWidget, Debug, Default)]
pub struct IntrinsicHeight {
    pub child: WidgetRef,
}

impl WidgetBuild for IntrinsicHeight {
    type Child = WidgetRef;

    fn build(&self, _: &mut BuildContext<Self>) -> Self::Child {
        build! {
            IntrinsicAxis {
                axis: Axis::Vertical,
                child: self.child.clone(),
            }
        }
    }
}
