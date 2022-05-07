use agui::canvas::{command::CanvasCommand, paint::Paint, texture::TextureId};
use glyph_brush_draw_cache::ab_glyph::FontArc;

use crate::render::context::RenderContext;

use super::{
    builder::{shape::LayerShapeBuilder, text::TextDrawCallBuilder, DrawCallBuilder},
    BrushData, CanvasBuffer, Layer,
};

#[derive(Debug, Default)]
pub struct CanvasBufferBuilder<'builder> {
    pub fonts: &'builder [FontArc],

    pub commands: Vec<CanvasCommand>,
}

impl CanvasBufferBuilder<'_> {
    pub fn build(self, ctx: &mut RenderContext) -> CanvasBuffer {
        // let mut color_data = vec![BrushData { color: [0.0; 4] }; self.paints.len()];

        // for (idx, paint) in self.paints.iter().enumerate() {
        //     brush_data[idx] = BrushData {
        //         color: paint.color.into(),
        //     };
        // }

        let mut canvas_buffer = CanvasBuffer {
            layers: vec![Layer::default()],
        };

        let mut layer_idx: usize = 0;
        let mut draw_call_builder: Option<Box<dyn DrawCallBuilder>> = None;

        for cmd in self.commands {
            // Check if the current layer builder can process the command, and finalize the build if not
            if let Some(builder) = draw_call_builder.as_ref() {
                if !builder.can_process(&cmd) {
                    // Add the draw call to the current layer

                    canvas_buffer.layers[layer_idx]
                        .draw_calls
                        .extend(builder.build(ctx, &brush_data));
                    draw_call_builder = None;
                }
            }

            match cmd {
                CanvasCommand::Layer { rect, shape, brush } => {
                    let paint = &self.paints[brush.idx()];

                    // Create a new layer and insert it after the current layer
                    let new_layer = Layer {
                        rect,
                        shape,
                        blend_mode: paint.blend_mode,

                        ..Layer::default()
                    };

                    canvas_buffer.layers.insert(layer_idx, new_layer);

                    // Switch to the new layer
                    layer_idx += 1;

                    continue;
                }

                CanvasCommand::Pop => {
                    // We can't pop beyond the layers owned by the canvas
                    if layer_idx == 0 {
                        panic!("can't pop layer, no layer to pop");
                    }

                    // Grab the previous layer
                    layer_idx -= 1;
                }

                CanvasCommand::Shape { .. } => {
                    if draw_call_builder.is_none() {
                        draw_call_builder =
                            Some(Box::new(LayerShapeBuilder::new(TextureId::default())));
                    }
                }

                CanvasCommand::Texture { texture_id, .. } => {
                    if draw_call_builder.is_none() {
                        draw_call_builder = Some(Box::new(LayerShapeBuilder::new(texture_id)));
                    }
                }

                CanvasCommand::Text { .. } => {
                    if draw_call_builder.is_none() {
                        draw_call_builder = Some(Box::new(TextDrawCallBuilder {
                            fonts: self.fonts,
                            ..TextDrawCallBuilder::default()
                        }));
                    }
                }

                cmd => {
                    tracing::error!("unknown command: {:?}", cmd);

                    continue;
                }
            }

            draw_call_builder.as_mut().unwrap().process(cmd);
        }

        if let Some(builder) = draw_call_builder.take() {
            canvas_buffer.layers[layer_idx]
                .draw_calls
                .extend(builder.build(ctx, &brush_data));
        }

        canvas_buffer
    }
}
