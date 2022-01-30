use std::collections::HashMap;

use agui::{
    canvas::{
        clipping::Clip,
        command::CanvasCommand,
        paint::{Brush, Paint},
    },
    unit::{Rect, Shape},
};

use crate::render::context::RenderContext;

use super::{
    builder::{shape::ShapeLayerBuilder, text::TextLayerBuilder, LayerBuilder},
    BrushData, CanvasBuffer,
};

#[derive(Debug, Default)]
pub struct CanvasBufferBuilder {
    pub clip: Option<(Rect, Clip, Shape)>,

    pub paint_map: HashMap<Paint, Brush>,

    pub commands: Vec<CanvasCommand>,
}

impl CanvasBufferBuilder {
    pub fn build(self, ctx: &mut RenderContext) -> CanvasBuffer {
        let mut brush_data = vec![BrushData { color: [0.0; 4] }; self.paint_map.len()];

        for (paint, brush) in self.paint_map {
            brush_data[brush.idx()] = BrushData {
                color: paint.color.into(),
            };
        }

        let mut canvas_buffer = CanvasBuffer {
            layers: Vec::default(),
        };

        let mut layer_builder: Option<Box<dyn LayerBuilder>> = None;

        for cmd in &self.commands {
            // Check if the current layer builder can process the command, and finalize the build if not
            if let Some(builder) = layer_builder.as_ref() {
                if !builder.can_process(cmd) {
                    canvas_buffer.layers.extend(builder.build(ctx, &brush_data));
                    layer_builder = None;
                }
            }

            match cmd {
                CanvasCommand::Clip { .. } => {}

                CanvasCommand::Shape { .. } => {
                    if layer_builder.is_none() {
                        layer_builder = Some(Box::new(ShapeLayerBuilder::default()));
                    }
                }

                CanvasCommand::Texture { .. } => {}

                CanvasCommand::Text { .. } => {
                    if layer_builder.is_none() {
                        layer_builder = Some(Box::new(TextLayerBuilder::default()));
                    }
                }

                cmd => panic!("unknown command: {:?}", cmd),
            }

            layer_builder.as_mut().unwrap().process(ctx, cmd);
        }

        if let Some(builder) = layer_builder.take() {
            canvas_buffer.layers.extend(builder.build(ctx, &brush_data));
        }

        canvas_buffer
    }
}
