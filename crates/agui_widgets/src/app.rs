use agui_core::{
    unit::{Key, Layout, Sizing, Units},
    widget::{BuildResult, WidgetBuilder, WidgetContext, WidgetRef},
};
use agui_macros::{build, Widget};

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
        let settings = ctx.use_global::<AppSettings, _>(AppSettings::default);

        let settings = settings.read();

        ctx.set_layout(build! {
            Layout {
                sizing: Sizing::Axis {
                    width: Units::Pixels(settings.width),
                    height: Units::Pixels(settings.height),
                }
            }
        });

        ctx.key(Key::single(), (&self.child).into()).into()
    }
}
