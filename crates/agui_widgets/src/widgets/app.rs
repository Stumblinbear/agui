use agui_core::{
    unit::{Layout, Sizing, Units},
    widget::{BuildContext, BuildResult, Widget, WidgetBuilder},
};

use crate::{plugins::global::GlobalPluginExt, state::window::WindowSize};

#[derive(Default)]
pub struct App {
    pub child: Widget,
}

impl WidgetBuilder for App {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        let window_size = ctx.get_global::<WindowSize>();

        let window_size = window_size.borrow();

        ctx.set_layout(Layout {
            sizing: Sizing::Axis {
                width: Units::Pixels(window_size.width),
                height: Units::Pixels(window_size.height),
            },
            ..Layout::default()
        });

        (&self.child).into()
    }
}
