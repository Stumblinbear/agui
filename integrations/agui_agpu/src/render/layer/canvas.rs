use std::collections::HashMap;

use agui::canvas::{
    command::CanvasCommand,
    paint::{Brush, Paint},
};
use glyph_brush_draw_cache::ab_glyph::FontArc;

use crate::render::context::RenderContext;

use super::{
    builder::{shape::ShapeLayerBuilder, text::TextLayerBuilder, LayerBuilder},
    BrushData, CanvasBuffer,
};

#[derive(Debug, Default)]
pub struct CanvasBufferBuilder<'builder> {
    pub fonts: &'builder [FontArc],

    pub paint_map: HashMap<Paint, Brush>,

    pub commands: Vec<CanvasCommand>,
}

impl CanvasBufferBuilder<'_> {
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

        for cmd in self.commands {
            // Check if the current layer builder can process the command, and finalize the build if not
            if let Some(builder) = layer_builder.as_ref() {
                if !builder.can_process(&cmd) {
                    canvas_buffer.layers.extend(builder.build(ctx, &brush_data));
                    layer_builder = None;
                }
            }

            match cmd {
                CanvasCommand::Layer { .. } => {}

                CanvasCommand::Pop => {}

                CanvasCommand::Shape { .. } => {
                    if layer_builder.is_none() {
                        layer_builder = Some(Box::new(ShapeLayerBuilder::default()));
                    }
                }

                CanvasCommand::Texture { .. } => {}

                CanvasCommand::Text { .. } => {
                    if layer_builder.is_none() {
                        layer_builder = Some(Box::new(TextLayerBuilder {
                            fonts: self.fonts,
                            ..TextLayerBuilder::default()
                        }));
                    }
                }

                cmd => {
                    tracing::error!("unknown command: {:?}", cmd);

                    continue;
                }
            }

            layer_builder.as_mut().unwrap().process(cmd);
        }

        if let Some(builder) = layer_builder.take() {
            canvas_buffer.layers.extend(builder.build(ctx, &brush_data));
        }

        canvas_buffer
    }
}
