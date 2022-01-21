use std::{io, mem};

use agpu::{
    winit::winit::event::{
        ElementState, Event as WinitEvent, MouseButton, MouseScrollDelta, WindowEvent,
    },
    Event, Frame, GpuProgram, RenderPipeline,
};

use agui::{
    engine::Engine,
    widget::{WidgetId, WidgetRef},
    widgets::{
        plugins::{hovering::HoveringPlugin, timeout::TimeoutPlugin},
        primitives::Fonts,
        state::{
            keyboard::{KeyCode, KeyState, Keyboard, KeyboardInput},
            mouse::{Mouse, MouseButtonState, Scroll, XY},
            window::{WindowFocus, WindowPosition, WindowSize},
        },
        AppSettings,
    },
};
use render::RenderContext;

mod render;

pub struct UI<'ui> {
}

impl<'ui> UI<'ui> {

    pub fn update(&mut self) -> bool {
        let mut events = Vec::new();

        self.engine.update(&mut events);

        if events.is_empty() {
            return false;
        }

        self.ctx.update();

        self.render.update(&self.engine, &events);

        self.render
            .take_dirty()
            .iter()
            .map(|render_id| (*render_id, self.render.get_tree().get(*render_id)))
            .for_each(|(render_id, node)| {
                if let Some(node) = node {
                    println!("{:?} {:?} {:?}", node.widget_id, render_id, node.rect);
                } else {
                    println!("destroyed");
                }
            });

        true
    }

    pub fn render(&self, mut frame: Frame) {
        // We complete rendering by first clearing the screen, then creating the layer buffer based on
        // clipping masks, before finally rendering the actual widgets through the added render passes.
        frame
            .render_pass_cleared("agui clear pass", 0x101010FF)
            .with_pipeline(&self.pipeline)
            .begin();
    }

    pub fn handle(&mut self, event: Event<'_, ()>, program: &GpuProgram) {
        
    }
}
