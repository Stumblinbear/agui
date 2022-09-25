use agui_core::{
    unit::{Layout, Sizing, Units},
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};

use crate::{plugins::global::GlobalPluginExt, state::window::WindowSize};

#[derive(Default, PartialEq)]
pub struct App {
    pub child: WidgetRef,
}

impl WidgetBuilder for App {
    fn build(&self, ctx: &mut BuildContext<Self>) -> BuildResult {
        let window_size = ctx.get_global::<WindowSize>();

        let window_size = window_size.borrow();

        BuildResult {
            layout: Layout {
                sizing: Sizing::Axis {
                    width: Units::Pixels(window_size.width),
                    height: Units::Pixels(window_size.height),
                },
                ..Layout::default()
            },

            children: vec![self.child.clone()],

            ..BuildResult::default()
        }
    }
}
