use agui_core::prelude::*;

use crate::{plugins::global::GlobalPluginExt, state::window::WindowSize};

#[derive(Debug, Default)]
pub struct App {
    pub child: Widget,
}

impl StatelessWidget for App {
    fn build(&self, ctx: &mut BuildContext<()>) -> BuildResult {
        let window_size = ctx.get_global::<WindowSize>();

        let window_size = window_size.borrow();

        ctx.set_layout(Layout {
            sizing: Sizing::Axis {
                width: Units::Pixels(window_size.width),
                height: Units::Pixels(window_size.height),
            },
            ..Layout::default()
        });

        ctx.key(Key::single(), self.child.clone()).into()
    }
}
