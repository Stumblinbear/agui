use agui_core::{
    canvas::paint::Paint,
    unit::Shape,
    widget::{BuildContext, BuildResult, WidgetBuilder, WidgetRef},
};
use agui_macros::Widget;

#[derive(Default, Widget)]
pub struct Clip {
    pub anti_alias: bool,
    pub shape: Shape,
    pub child: WidgetRef,
}

impl WidgetBuilder for Clip {
    fn build(&self, ctx: &mut BuildContext) -> BuildResult {
        let anti_alias = self.anti_alias;
        let shape = self.shape.clone();

        ctx.on_draw(move |canvas| {
            let brush = canvas.new_brush(Paint {
                anti_alias,
                ..Paint::default()
            });

            canvas.start_layer(brush, shape.clone());
        });

        (&self.child).into()
    }
}
