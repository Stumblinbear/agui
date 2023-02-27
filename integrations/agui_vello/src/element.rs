use agui::{
    element::ElementId,
    render::canvas::{Canvas, CanvasCommand},
    unit::{Offset, Rect},
    util::tree::new_key_type,
};
use vello::{
    glyph::{
        pinot::{FontRef, TableProvider},
        GlyphContext,
    },
    kurbo::{Affine, PathEl, Vec2},
    peniko::{Brush, Color, Fill, Mix},
    SceneBuilder, SceneFragment,
};

new_key_type! {
    pub struct CanvasId;
}

#[derive(Default)]
pub(crate) struct RenderElement {
    /// This is the layer that this render element belongs to
    pub head_target: Option<ElementId>,

    pub offset: Offset,

    pub canvas: CanvasElement,
}

#[derive(Default)]
pub(crate) struct CanvasElement {
    pub offset: Offset,

    pub fragment: SceneFragment,

    pub children: Vec<LayerElement>,
    pub tail: Option<Box<LayerElement>>,
}

impl CanvasElement {
    pub fn update(&mut self, gcx: &mut GlyphContext, canvas: Option<Canvas>) {
        let Some(canvas) = canvas else {
            self.fragment = SceneFragment::default();
            self.children.clear();
            self.tail = None;

            return;
        };

        if canvas.head.is_empty() {
            self.fragment = SceneFragment::default();
        } else {
            self.update_head(gcx, &canvas.head);
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
                },
            };

            layer_element.update(gcx, Some(tail.canvas));

            self.tail = Some(Box::new(layer_element));
        }
    }

    fn update_head(&mut self, gcx: &mut GlyphContext, commands: &[CanvasCommand]) {
        let mut sb = SceneBuilder::for_fragment(&mut self.fragment);

        const FONT_REF: FontRef = FontRef {
            data: include_bytes!("../examples/fonts/DejaVuSans.ttf"),
            offset: 0,
        };

        for command in commands {
            match command {
                CanvasCommand::Shape { rect, shape, color } => {
                    sb.fill(
                        Fill::NonZero,
                        Affine::translate((rect.left as f64, rect.top as f64)),
                        Color::rgba(
                            color.red as f64,
                            color.green as f64,
                            color.blue as f64,
                            color.alpha as f64,
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
                    rect,
                    color,
                    font,
                    text,
                    ..
                } => {
                    let transform =
                        Affine::translate((rect.left as f64, (font.size + rect.top) as f64));

                    let brush = &Brush::Solid(Color::rgba(
                        color.red as f64,
                        color.green as f64,
                        color.blue as f64,
                        color.alpha as f64,
                    ));

                    if let Some(cmap) = FONT_REF.cmap() {
                        if let Some(hmtx) = FONT_REF.hmtx() {
                            let upem = FONT_REF
                                .head()
                                .map(|head| head.units_per_em())
                                .unwrap_or(1000) as f64;

                            let scale = font.size as f64 / upem;

                            let vars: [(pinot::types::Tag, f32); 0] = [];

                            let mut provider =
                                gcx.new_provider(&FONT_REF, None, font.size, false, vars);

                            let hmetrics = hmtx.hmetrics();

                            let default_advance = hmetrics
                                .get(hmetrics.len().saturating_sub(1))
                                .map(|h| h.advance_width)
                                .unwrap_or(0);

                            let mut pen_x = 0f64;

                            for ch in text.chars() {
                                let gid = cmap.map(ch as u32).unwrap_or(0);

                                let advance = hmetrics
                                    .get(gid as usize)
                                    .map(|h| h.advance_width)
                                    .unwrap_or(default_advance)
                                    as f64
                                    * scale;

                                if let Some(glyph) = provider.get(gid, Some(brush)) {
                                    let xform = transform
                                        * Affine::translate((pen_x, 0.0))
                                        * Affine::scale_non_uniform(1.0, -1.0);

                                    sb.append(&glyph, Some(xform));
                                }

                                pen_x += advance;
                            }
                        }
                    }
                }

                cmd => {
                    tracing::error!("unknown command: {:?}", cmd);

                    continue;
                }
            }
        }

        sb.finish();
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

pub(crate) struct LayerElement {
    pub rect: Rect,

    pub canvas: CanvasElement,
}

impl LayerElement {
    pub fn update(&mut self, gcx: &mut GlyphContext, canvas: Option<Canvas>) {
        self.canvas.update(gcx, canvas);
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
