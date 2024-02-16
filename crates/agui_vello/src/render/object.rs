use agui_core::{
    render::canvas::{command::CanvasCommand, paint::Paint, Canvas},
    unit::{Offset, Rect, Size},
};
use rustc_hash::FxHashMap;
use vello::{
    glyph::{
        skrifa::{GlyphId, MetadataProvider},
        Glyph,
    },
    kurbo::{Affine, PathEl, Vec2},
    peniko::{Color, Fill, Mix},
    Scene,
};

use crate::renderer::fonts::VelloFonts;

#[derive(Debug, Default)]
pub struct VelloRenderObject {
    // /// This is the layer that this render object belongs to
    // pub head_target: Option<RenderObjectId>,
    pub size: Size,

    pub offset: Offset,

    pub canvas: VelloCanvasObject,
}

#[derive(Default)]
pub struct VelloCanvasObject {
    pub offset: Offset,

    pub fragment: Scene,

    pub children: Vec<LayerObject>,
    pub tail: Option<Box<LayerObject>>,

    pub paints: Vec<Paint>,
    pub glyph_cache: FxHashMap<(GlyphId, usize), Option<Scene>>,
}

impl std::fmt::Debug for VelloCanvasObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VelloCanvasObject")
            .field("offset", &self.offset)
            .finish_non_exhaustive()
    }
}

impl VelloCanvasObject {
    pub fn update(&mut self, fonts: &mut VelloFonts, canvas: Canvas) {
        // TODO: only invalidate paints that are different
        if self.paints.len() != canvas.paints.len() || self.paints != canvas.paints {
            self.paints = canvas.paints;

            // If our paints have changed, we need to invalidate the glyph cache
            // TODO: only invalidate glpyhs whose paint has changed
            self.glyph_cache.clear();
        }

        if canvas.head.is_empty() {
            self.fragment.reset();
        } else {
            self.update_head(fonts, canvas.head);
        }

        for _ in canvas.children {
            println!("child");
        }

        if let Some(tail) = canvas.tail {
            let mut layer = LayerObject {
                rect: tail.offset & tail.canvas.size,

                canvas: VelloCanvasObject {
                    offset: tail.offset,

                    fragment: Scene::new(),

                    children: Vec::new(),
                    tail: None,

                    paints: Vec::new(),
                    glyph_cache: FxHashMap::default(),
                },
            };

            layer.update(fonts, tail.canvas);

            self.tail = Some(Box::new(layer));
        } else {
            self.tail = None;
        }
    }

    fn update_head(&mut self, fonts: &mut VelloFonts, commands: Vec<CanvasCommand>) {
        self.fragment.reset();

        for command in commands {
            match command {
                CanvasCommand::Shape {
                    paint_idx,
                    rect,
                    shape,
                } => {
                    let paint = &self.paints[paint_idx];

                    self.fragment.fill(
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
                    texture,
                    tex_bounds,
                } => {
                    tracing::info!("texture: {:?}", texture);
                }

                CanvasCommand::Text {
                    paint_idx,
                    rect,
                    text_style,
                    text,
                    ..
                } => {
                    // TODO: should we handle handle text wrapping here or in the render object?

                    let paint = &self.paints[paint_idx];

                    let font = fonts.get_or_insert(text_style.font);
                    let font_ref = VelloFonts::to_font_ref(font).expect("failed to get font ref");

                    let transform = Affine::translate((
                        rect.left as f64,
                        rect.top as f64 + text_style.size as f64,
                    ));

                    let glyph_brush = vello::peniko::Brush::Solid(Color::rgba(
                        paint.color.red as f64,
                        paint.color.green as f64,
                        paint.color.blue as f64,
                        paint.color.alpha as f64,
                    ));

                    let axes = font_ref.axes();
                    let font_size = vello::skrifa::instance::Size::new(text_style.size);
                    let var_loc = axes.location(&[] as &[(&str, f32)]);
                    let charmap = font_ref.charmap();
                    let metrics = font_ref.metrics(font_size, &var_loc);
                    let line_height = metrics.ascent - metrics.descent + metrics.leading;
                    let glyph_metrics = font_ref.glyph_metrics(font_size, &var_loc);
                    let mut pen_x = 0f32;
                    let mut pen_y = 0f32;

                    self.fragment
                        .draw_glyphs(font)
                        .font_size(text_style.size)
                        .transform(transform)
                        .glyph_transform(None)
                        .normalized_coords(var_loc.coords())
                        .brush(&glyph_brush)
                        .draw(
                            Fill::NonZero,
                            text.chars().filter_map(|ch| {
                                if ch == '\n' {
                                    pen_y += line_height;
                                    pen_x = 0.0;
                                    return None;
                                }

                                let gid = charmap.map(ch).unwrap_or_default();
                                let advance = glyph_metrics.advance_width(gid).unwrap_or_default();

                                // Naive wrapping (doesn't account for word boundaries)
                                if pen_x + advance > rect.width {
                                    pen_y += line_height;
                                    pen_x = 0.0;
                                }

                                let x = pen_x;
                                pen_x += advance;

                                Some(Glyph {
                                    id: gid.to_u16() as u32,

                                    x,
                                    y: pen_y,
                                })
                            }),
                        );

                    // let fello_size = vello::fello::Size::new(text_style.size);
                    // let charmap = font.charmap();
                    // let metrics = font.metrics(fello_size, Default::default());
                    // let line_height = metrics.ascent - metrics.descent + metrics.leading;
                    // let glyph_metrics = font.glyph_metrics(fello_size, Default::default());
                    // let mut pen_x = 0f64;
                    // let mut pen_y = 0f64;
                    // let vars: [(&str, f32); 0] = [];
                    // let mut provider = fonts.new_provider(&font, text_style.size, false, vars);

                    // for ch in text.chars() {
                    //     if ch == '\n' {
                    //         pen_y += line_height as f64;
                    //         pen_x = 0.0;
                    //         continue;
                    //     }

                    //     let gid = charmap.map(ch).unwrap_or_default();
                    //     let advance = glyph_metrics.advance_width(gid).unwrap_or_default() as f64;

                    //     // Getting the glyph from the provider is expensive
                    //     if let Some(glyph) = self
                    //         .glyph_cache
                    //         .entry((gid, *paint_idx))
                    //         .or_insert_with(|| provider.get(gid.to_u16(), Some(glyph_brush)))
                    //     {
                    //         let xform = transform
                    //             * Affine::translate((pen_x, pen_y + text_style.size as f64))
                    //             * Affine::scale_non_uniform(1.0, -1.0);

                    //         sb.append(glyph, Some(xform));
                    //     }

                    //     pen_x += advance;
                    // }
                }

                cmd => {
                    tracing::error!("unknown command: {:?}", cmd);

                    continue;
                }
            }
        }
    }

    pub fn begin(&self, transform: Affine, scene: &mut Scene) {
        let transform =
            transform * Affine::translate(Vec2::new(self.offset.x as f64, self.offset.y as f64));

        scene.append(&self.fragment, Some(transform));

        for child in &self.children {
            child.begin(transform, scene);
            child.end(transform, scene);
        }

        if let Some(tail) = &self.tail {
            tail.begin(transform, scene);
        }
    }

    pub fn end(&self, transform: Affine, scene: &mut Scene) {
        if let Some(tail) = &self.tail {
            tail.end(transform, scene);
        }
    }
}

pub struct LayerObject {
    pub rect: Rect,

    pub canvas: VelloCanvasObject,
}

impl LayerObject {
    pub fn update(&mut self, fonts: &mut VelloFonts, canvas: Canvas) {
        self.canvas.update(fonts, canvas);
    }

    pub fn begin(&self, transform: Affine, scene: &mut Scene) {
        let transform =
            transform * Affine::translate((self.rect.left as f64, self.rect.top as f64));

        scene.push_layer(
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

        self.canvas.begin(transform, scene);
    }

    pub fn end(&self, transform: Affine, scene: &mut Scene) {
        self.canvas.end(transform, scene);

        scene.pop_layer();
    }
}
