use std::hash::Hash;

use agui_core::{
    element::ElementId,
    render::{
        canvas::{Canvas, CanvasCommand},
        Paint,
    },
    unit::{Offset, Rect},
    util::tree::new_key_type,
};
use rustc_hash::FxHashMap;
use vello::{
    fello::{GlyphId, MetadataProvider},
    kurbo::{Affine, PathEl, Vec2},
    peniko::{Color, Fill, Mix},
    SceneBuilder, SceneFragment,
};

use crate::fonts::VelloFonts;

new_key_type! {
    pub struct CanvasId;
}

#[derive(Default)]
pub struct RenderElement {
    /// This is the layer that this render element belongs to
    pub head_target: Option<ElementId>,

    pub offset: Offset,

    pub canvas: CanvasElement,
}

#[derive(Default)]
pub struct CanvasElement {
    pub offset: Offset,

    pub fragment: SceneFragment,

    pub children: Vec<LayerElement>,
    pub tail: Option<Box<LayerElement>>,

    pub paints: Vec<Paint>,
    pub glyph_cache: FxHashMap<(GlyphId, usize), Option<SceneFragment>>,
}

impl CanvasElement {
    pub fn update(&mut self, fonts: &mut VelloFonts<'_>, canvas: Option<Canvas>) {
        let Some(canvas) = canvas else {
            self.fragment = SceneFragment::default();
            self.children.clear();
            self.tail = None;

            self.paints.clear();
            self.glyph_cache.clear();

            return;
        };

        // TODO: only invalidate paints that are different
        if self.paints.len() != canvas.paints.len() || self.paints != canvas.paints {
            self.paints = canvas.paints;

            // If our paints have changed, we need to invalidate the glyph cache
            // TODO: only invalidate glpyhs whose paint has changed
            self.glyph_cache.clear();
        }

        if canvas.head.is_empty() {
            self.fragment = SceneFragment::default();
        } else {
            self.update_head(fonts, &canvas.head);
        }

        for child in canvas.children {
            println!("child: {:?}", child);
        }

        if let Some(tail) = canvas.tail {
            let mut layer_element = LayerElement {
                rect: tail.offset & tail.canvas.size,

                canvas: CanvasElement {
                    offset: tail.offset,

                    fragment: SceneFragment::default(),

                    children: Vec::new(),
                    tail: None,

                    paints: Vec::new(),
                    glyph_cache: FxHashMap::default(),
                },
            };

            layer_element.update(fonts, Some(tail.canvas));

            self.tail = Some(Box::new(layer_element));
        }
    }

    fn update_head(&mut self, fonts: &mut VelloFonts<'_>, commands: &[CanvasCommand]) {
        let mut sb = SceneBuilder::for_fragment(&mut self.fragment);

        for command in commands {
            match command {
                CanvasCommand::Shape {
                    paint_idx,
                    rect,
                    shape,
                } => {
                    let paint = &self.paints[*paint_idx];

                    sb.fill(
                        Fill::NonZero,
                        Affine::translate((rect.left as f64, rect.top as f64)),
                        Color::rgba(
                            paint.color.red as f64,
                            paint.color.green as f64,
                            paint.color.blue as f64,
                            paint.color.alpha as f64,
                        ),
                        None,
                        &[
                            PathEl::LineTo((0.0, 0.0).into()),
                            PathEl::LineTo((rect.width as f64, 0.0).into()),
                            PathEl::LineTo((rect.width as f64, rect.height as f64).into()),
                            PathEl::LineTo((0.0, rect.height as f64).into()),
                            PathEl::ClosePath,
                        ],
                    );
                }

                CanvasCommand::Texture {
                    rect,
                    shape,
                    texture_id,
                    tex_bounds,
                } => {
                    tracing::info!("texture: {:?}", texture_id);
                }

                CanvasCommand::Text {
                    paint_idx,
                    rect,
                    font_style,
                    text,
                    ..
                } => {
                    let paint = &self.paints[*paint_idx];

                    if let Some(font) = fonts.get_or_default(font_style.font) {
                        let transform = Affine::translate((rect.left as f64, rect.top as f64));

                        let glyph_brush = &vello::peniko::Brush::Solid(Color::rgba(
                            paint.color.red as f64,
                            paint.color.green as f64,
                            paint.color.blue as f64,
                            paint.color.alpha as f64,
                        ));

                        let fello_size = vello::fello::Size::new(font_style.size);
                        let charmap = font.charmap();
                        let metrics = font.metrics(fello_size, Default::default());
                        let line_height = metrics.ascent - metrics.descent + metrics.leading;
                        let glyph_metrics = font.glyph_metrics(fello_size, Default::default());
                        let mut pen_x = 0f64;
                        let mut pen_y = 0f64;
                        let vars: [(&str, f32); 0] = [];
                        let mut provider =
                            fonts.new_provider(&font, None, font_style.size, false, vars);

                        for ch in text.chars() {
                            if ch == '\n' {
                                pen_y += line_height as f64;
                                pen_x = 0.0;
                                continue;
                            }

                            let gid = charmap.map(ch).unwrap_or_default();
                            let advance =
                                glyph_metrics.advance_width(gid).unwrap_or_default() as f64;

                            // Getting the glyph from the provider is expensive
                            if let Some(glyph) = self
                                .glyph_cache
                                .entry((gid, *paint_idx))
                                .or_insert_with(|| provider.get(gid.to_u16(), Some(glyph_brush)))
                            {
                                let xform = transform
                                    * Affine::translate((pen_x, pen_y + font_style.size as f64))
                                    * Affine::scale_non_uniform(1.0, -1.0);

                                sb.append(glyph, Some(xform));
                            }

                            pen_x += advance;
                        }
                    }
                }

                cmd => {
                    tracing::error!("unknown command: {:?}", cmd);

                    continue;
                }
            }
        }
    }

    pub fn begin(&self, transform: Affine, sb: &mut SceneBuilder) {
        let transform =
            transform * Affine::translate(Vec2::new(self.offset.x as f64, self.offset.y as f64));

        sb.append(&self.fragment, Some(transform));

        for child in &self.children {
            child.begin(transform, sb);
            child.end(transform, sb);
        }

        if let Some(tail) = &self.tail {
            tail.begin(transform, sb);
        }
    }

    pub fn end(&self, transform: Affine, sb: &mut SceneBuilder) {
        if let Some(tail) = &self.tail {
            tail.end(transform, sb);
        }
    }
}

pub struct LayerElement {
    pub rect: Rect,

    pub canvas: CanvasElement,
}

impl LayerElement {
    pub fn update(&mut self, fonts: &mut VelloFonts<'_>, canvas: Option<Canvas>) {
        self.canvas.update(fonts, canvas);
    }

    pub fn begin(&self, transform: Affine, sb: &mut SceneBuilder) {
        let transform =
            transform * Affine::translate((self.rect.left as f64, self.rect.top as f64));

        sb.push_layer(
            Mix::Clip,
            1.0,
            transform,
            &[
                PathEl::LineTo((0.0, 0.0).into()),
                PathEl::LineTo((self.rect.width as f64, 0.0).into()),
                PathEl::LineTo((self.rect.width as f64, self.rect.height as f64).into()),
                PathEl::LineTo((0.0, self.rect.height as f64).into()),
                PathEl::ClosePath,
            ],
        );

        self.canvas.begin(transform, sb);
    }

    pub fn end(&self, transform: Affine, sb: &mut SceneBuilder) {
        self.canvas.end(transform, sb);

        sb.pop_layer();
    }
}
