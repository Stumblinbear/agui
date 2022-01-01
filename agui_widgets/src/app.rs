use agui_core::{
    context::WidgetContext,
    layout::Layout,
    unit::{Key, Sizing, Units},
    widget::{BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::{build, Widget};

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
    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        let settings = ctx.get_global_or::<AppSettings, _>(AppSettings::default);

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
