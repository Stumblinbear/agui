use agui_core::{
    unit::{Key, Layout, Sizing, Units},
    widget::{BuildResult, WidgetBuilder, WidgetContext, WidgetRef},
};
use agui_macros::{build, Widget};

use crate::state::window::WindowSize;

#[derive(Debug)]
pub struct AppSettings {
    pub width: f32,
    pub height: f32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            width: 256.0,
            height: 256.0,
        }
    }
}

#[derive(Default, Widget)]
pub struct App {
    pub child: WidgetRef,
}

impl WidgetBuilder for App {
    fn build(&self, ctx: &mut WidgetContext) -> BuildResult {
        if let Some(window_size) = ctx.try_use_global::<WindowSize>() {
            let window_size = window_size.read();

            ctx.set_layout(build! {
                Layout {
                    sizing: Sizing::Axis {
                        width: Units::Pixels(window_size.width),
                        height: Units::Pixels(window_size.height),
                    }
                }
            });
        }

        ctx.key(Key::single(), (&self.child).into()).into()
    }
}
