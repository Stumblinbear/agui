use agui_core::{
    callback::Callback,
    render::{CanvasPainter, Paint},
    unit::{Color, Layout, LayoutType},
    widget::{
        BuildContext, Children, ContextStatefulWidget, ContextWidgetMut, LayoutContext,
        LayoutResult, PaintContext, WidgetRef, WidgetState, WidgetView,
    },
};
use agui_macros::StatefulWidget;

use crate::GestureDetector;

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonStyle {
    pub normal: Color,
    pub disabled: Color,
    pub hovered: Color,
    pub pressed: Color,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            normal: Color::from_rgb((1.0, 1.0, 1.0)),
            disabled: Color::from_rgb((0.3, 0.3, 0.3)),
            hovered: Color::from_rgb((0.7, 0.7, 0.7)),
            pressed: Color::from_rgb((0.5, 0.5, 0.5)),
        }
    }
}

#[derive(Debug, Default)]
pub struct ButtonState {
    pressed: bool,
    hovered: bool,
    disabled: bool,
}

#[derive(StatefulWidget, Default, PartialEq)]
pub struct Button {
    pub layout: Layout,
    pub style: Option<ButtonStyle>,

    pub on_pressed: Callback<()>,

    pub child: WidgetRef,
}

impl WidgetState for Button {
    type State = ButtonState;

    fn create_state(&self) -> Self::State {
        ButtonState::default()
    }
}

impl WidgetView for Button {
    fn layout(&self, _: &mut LayoutContext<Self>) -> LayoutResult {
        LayoutResult {
            layout_type: LayoutType::default(),

            layout: Layout::clone(&self.layout),
        }
    }

    fn build(&self, ctx: &mut BuildContext<Self>) -> Children {
        let on_hover = ctx.callback::<bool, _>(|ctx, arg| {
            if ctx.hovered != *arg {
                ctx.set_state(|state| {
                    state.hovered = *arg;
                })
            }
        });

        let on_pressed = ctx.callback::<bool, _>(|ctx, arg| {
            if ctx.pressed && !arg {
                ctx.on_pressed.call(());
            }

            ctx.set_state(|state| {
                state.pressed = *arg;
            })
        });

        Children::from([GestureDetector {
            on_hover,
            on_pressed,

            child: (&self.child).into(),

            ..Default::default()
        }])
    }

    fn paint(&self, ctx: &mut PaintContext<Self>, mut canvas: CanvasPainter) {
        let style = self.style.clone().unwrap_or_default();

        let color = if ctx.disabled {
            style.disabled
        } else if ctx.pressed {
            style.pressed
        } else if ctx.hovered {
            style.hovered
        } else {
            style.normal
        };

        canvas.draw_rect(&Paint {
            color,
            ..Paint::default()
        });
    }
}
