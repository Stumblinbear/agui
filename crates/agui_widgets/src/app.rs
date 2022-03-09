use agui_core::{
    unit::{Key, Layout, Sizing, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::{build, Widget};

use crate::state::window::WindowSize;

#[derive(Default, Widget)]
pub struct App {
    pub child: WidgetRef,
}

impl WidgetBuilder for App {
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        let window_size = ctx.use_global(WindowSize::default);

        ctx.set_layout(build! {
            Layout {
                sizing: Sizing::Axis {
                    width: Units::Pixels(window_size.width),
                    height: Units::Pixels(window_size.height),
                }
            }
        });

        ctx.key(Key::single(), (&self.child).into()).into()
    }
}
