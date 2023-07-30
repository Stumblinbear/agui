use agui::{
    element::ElementId,
    render::canvas::{Canvas, CanvasCommand},
    unit::{Offset, Rect},
    util::tree::new_key_type,
};
use vello::{
    fello::{raw::FontRef, MetadataProvider},
    glyph::GlyphContext,
    kurbo::{Affine, PathEl, Vec2},
    peniko::{Brush, Color, Fill, Font, Mix},
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

        let default_font =
            FontRef::new(include_bytes!("../examples/fonts/DejaVuSans.ttf")).unwrap();

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
                    let transform = Affine::translate((rect.left as f64, rect.top as f64));

                    let brush = &Brush::Solid(Color::rgba(
                        color.red as f64,
                        color.green as f64,
                        color.blue as f64,
                        color.alpha as f64,
                    ));

                    let fello_size = vello::fello::Size::new(font.size);
                    let charmap = default_font.charmap();
                    let metrics = default_font.metrics(fello_size, Default::default());
                    let line_height = metrics.ascent - metrics.descent + metrics.leading;
                    let glyph_metrics = default_font.glyph_metrics(fello_size, Default::default());
                    let mut pen_x = 0f64;
                    let mut pen_y = 0f64;
                    let vars: [(&str, f32); 0] = [];
                    let mut provider =
                        gcx.new_provider(&default_font, None, font.size, false, vars);
                    for ch in text.chars() {
                        if ch == '\n' {
                            pen_y += line_height as f64;
                            pen_x = 0.0;
                            continue;
                        }
                        let gid = charmap.map(ch).unwrap_or_default();
                        let advance = glyph_metrics.advance_width(gid).unwrap_or_default() as f64;
                        if let Some(glyph) = provider.get(gid.to_u16(), Some(brush)) {
                            let xform = transform
                                * Affine::translate((pen_x, pen_y + font.size as f64))
                                * Affine::scale_non_uniform(1.0, -1.0);
                            sb.append(&glyph, Some(xform));
                        }
                        pen_x += advance;
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

fn to_font_ref(font: &Font) -> Option<FontRef<'_>> {
    use vello::fello::raw::FileRef;
    let file_ref = FileRef::new(font.data.as_ref()).ok()?;
    match file_ref {
        FileRef::Font(font) => Some(font),
        FileRef::Collection(collection) => collection.get(font.index).ok(),
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
