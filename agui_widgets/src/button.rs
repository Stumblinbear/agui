use agui_core::{
    render::color::Color,
    state::mouse::MousePosition,
    unit::{Layout, Sizing},
    BuildResult, WidgetContext, WidgetImpl, WidgetRef,
};
use agui_macros::{build, Widget};
use agui_primitives::{Quad, Text};

#[derive(Default, Widget)]
#[widget(layout = "row")]
pub struct Button {
    pub layout: Layout,

    pub color: Color,
    
    pub child: WidgetRef,
}

impl WidgetImpl for Button {
    fn layout(&self) -> Option<&Layout> {
        Some(&self.layout)
    }

    fn build(&self, ctx: &WidgetContext) -> BuildResult {
        let _hovering = ctx.computed(|ctx| {
            let mouse = ctx.get_state::<MousePosition>();

            let mouse_pos = mouse.read();

            mouse_pos.x > 50.0
        });

        BuildResult::One(build! {
            Quad {
                layout: Layout { sizing: Sizing::Fill },
                color: Color::White,
                child: Text {
                    text: String::from("")
                }
            }
        })
    }
}
