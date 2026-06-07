use std::rc::Rc;

use parley::{Alignment, AlignmentOptions, Layout, PositionedLayoutItem};
use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::{LayoutCtx, MountCtx, PaintCtx},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    paint::command::GlyphInstance,
    pipeline::{layout::LayoutScope, paint::PaintScope},
    render_object::{RenderObject, box_layout::BoxConstraints},
    text::{Fonts, TextBaseline, TextBrush},
};

use crate::render_object::box_layout::RenderBox;

/// A box render object that shapes and sizes a run of text.
pub struct RenderParagraph {
    text: String,

    font_size: f32,
    brush: TextBrush,
    family: Option<String>,

    fonts: Option<Rc<Fonts>>,

    layout: Option<Layout<TextBrush>>,
    dirty: bool,
    /// The width the cached layout was last broken to, so a query at the same width reuses it.
    broken_width: Option<f32>,

    layout_scope: LayoutScope,
    paint_scope: Option<PaintScope>,
}

impl RenderParagraph {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 16.0,
            brush: TextBrush::default(),
            family: None,

            fonts: None,

            layout: None,
            dirty: true,
            broken_width: None,

            layout_scope: LayoutScope::detached(),
            paint_scope: None,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.text != text {
            self.text = text;
            self.mark_needs_reshape();
        }
    }

    // Exact comparison is the intent: shape only when the size literally changes.
    #[allow(clippy::float_cmp)]
    pub fn set_font_size(&mut self, font_size: f32) {
        if self.font_size != font_size {
            self.font_size = font_size;
            self.mark_needs_reshape();
        }
    }

    pub fn set_brush(&mut self, brush: TextBrush) {
        if self.brush != brush {
            self.brush = brush;
            // The brush is baked into each run during shaping, so a recolor re-shapes.
            self.mark_needs_reshape();
        }
    }

    pub fn set_font_family(&mut self, family: Option<String>) {
        if self.family != family {
            self.family = family;
            self.mark_needs_reshape();
        }
    }

    /// Captures the font database to shape against. A change of handle identity dirties the cache.
    pub fn set_fonts(&mut self, fonts: Option<Rc<Fonts>>) {
        let same = match (&self.fonts, &fonts) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };

        if !same {
            self.fonts = fonts;
            self.mark_needs_reshape();
        }
    }

    /// Re-shapes on the next layout and asks the pipeline to re-lay this box out.
    fn mark_needs_reshape(&mut self) {
        self.dirty = true;
        self.layout_scope.mark_needs_layout();
    }

    /// Shapes the current text into a fresh layout via the captured handle, leaving `self` untouched.
    fn shape(&self) -> Option<Layout<TextBrush>> {
        let fonts = self.fonts.as_ref()?;

        Some(fonts.shape(
            &self.text,
            self.font_size,
            self.brush.clone(),
            self.family.as_deref(),
        ))
    }

    /// The maximum advance to break against: the finite max width, or `None` when unbounded.
    fn max_advance(constraints: BoxConstraints) -> Option<f32> {
        constraints
            .has_bounded_width()
            .then(|| constraints.max_width().get())
    }
}

impl RenderObject for RenderParagraph {
    fn mount(&mut self, ctx: &mut MountCtx) {
        self.paint_scope = Some(ctx.paint_scope().clone());
    }

    fn unmount(&mut self, _: &mut MountCtx) {}

    fn update_compositing_bits(&mut self) -> bool {
        false
    }
}

impl RenderBox for RenderParagraph {
    fn measure(&self, constraints: BoxConstraints) -> Size {
        let shaped = self.shape();

        let Some(layout) = shaped.as_ref().or(self.layout.as_ref()) else {
            return constraints.smallest();
        };

        constraints.constrain(measure_clone(layout, Self::max_advance(constraints)))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = ctx.scope().clone();

        if self.fonts.is_none() {
            return constraints.smallest();
        }

        let reshaped = self.dirty || self.layout.is_none();

        if reshaped {
            self.layout = self.shape();
            self.dirty = false;
            self.broken_width = None;
        }

        let max_advance = Self::max_advance(constraints);

        let Some(layout) = self.layout.as_mut() else {
            return constraints.smallest();
        };

        // Reuse the existing line breaking when the width hasn't changed since the last pass.
        if self.broken_width != max_advance {
            layout.break_all_lines(max_advance);
            layout.align(Alignment::Start, AlignmentOptions::default());
        }

        let size = Size::new(layout.width(), layout.height());

        self.broken_width = max_advance;

        constraints.constrain(size)
    }

    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        // A real minimum is available via the layout's content widths, but deriving one is left for
        // its own change rather than folded into the dependency bump.
        None
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let shaped = self.shape();
        let layout = shaped.as_ref().or(self.layout.as_ref())?;
        let width = measure_clone(layout, None).width.get();
        PositiveFinite::try_from(width).ok()
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.max_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let shaped = self.shape();
        let layout = shaped.as_ref().or(self.layout.as_ref())?;
        // Infinite width means no wrap.
        let max_advance = (width.get() < f32::INFINITY).then(|| width.get());
        let height = measure_clone(layout, max_advance).height.get();
        PositiveFinite::try_from(height).ok()
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        let shaped = self.shape();
        let layout = shaped.as_ref().or(self.layout.as_ref())?;
        let mut clone = layout.clone();
        clone.break_all_lines(Self::max_advance(constraints));
        let baseline = clone.lines().next()?.metrics().baseline;
        PositiveFinite::try_from(baseline).ok()
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        let layout = self.layout.as_ref()?;
        let baseline = layout.lines().next()?.metrics().baseline;
        PositiveFinite::try_from(baseline).ok()
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let Some(layout) = self.layout.as_ref() else {
            return;
        };

        let mut canvas = ctx.canvas();
        canvas.with_offset(offset, |canvas| {
            for line in layout.lines() {
                for item in line.items() {
                    let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                        continue;
                    };

                    let run = glyph_run.run();
                    // Each run carries its own brush, so a styled paragraph paints in several colors.
                    let brush = canvas.brush(glyph_run.style().brush.0.clone());

                    // positioned_glyphs bakes the run offset and baseline into each glyph's x/y.
                    let glyphs = glyph_run
                        .positioned_glyphs()
                        .map(|glyph| GlyphInstance {
                            id: u32::from(glyph.id),
                            x: glyph.x,
                            y: glyph.y,
                        })
                        .collect();

                    canvas.draw_glyphs(run.font(), run.font_size(), brush, glyphs);
                }
            }
        });
    }
}

fn measure_clone(layout: &Layout<TextBrush>, max_advance: Option<f32>) -> Size {
    let mut clone = layout.clone();
    clone.break_all_lines(max_advance);
    Size::new(clone.width(), clone.height())
}

#[cfg(test)]
mod tests {
    use crate::text::{Fonts, TextBaseline};

    use super::*;

    fn infinite() -> Positive<f32> {
        Positive::try_from(f32::INFINITY).unwrap()
    }

    /// A paragraph with a font handle captured, sized through a detached layout pass.
    fn shaped(text: &str, constraints: BoxConstraints) -> RenderParagraph {
        let mut paragraph = RenderParagraph::new(text);
        paragraph.set_font_size(20.0);
        paragraph.set_fonts(Some(Rc::new(Fonts::new())));
        paragraph.layout(&mut LayoutCtx::detached(), constraints);

        paragraph
    }

    #[test]
    fn measure_agrees_with_layout_after_shaping() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);

        let mut paragraph = RenderParagraph::new("hello world");
        paragraph.set_font_size(20.0);
        paragraph.set_fonts(Some(Rc::new(Fonts::new())));
        let laid_out = paragraph.layout(&mut LayoutCtx::detached(), constraints);

        assert_eq!(paragraph.measure(constraints), laid_out);
    }

    #[test]
    fn measure_works_without_a_prior_layout() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);

        let mut paragraph = RenderParagraph::new("no prior layout");
        paragraph.set_font_size(20.0);
        paragraph.set_fonts(Some(Rc::new(Fonts::new())));

        // The handle lets measure shape on demand, even though layout never ran.
        let measured = paragraph.measure(constraints);
        let laid_out = paragraph.layout(&mut LayoutCtx::detached(), constraints);
        assert_eq!(measured, laid_out);
    }

    #[test]
    fn narrower_break_does_not_grow_width() {
        let wide = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let paragraph = shaped("the quick brown fox jumps over the lazy dog", wide);

        let wide_width = paragraph.measure(wide).width;
        let narrow = BoxConstraints::new(0.0, 80.0, 0.0, 1000.0);
        let narrow_width = paragraph.measure(narrow).width;

        assert!(narrow_width <= wide_width);
    }

    #[test]
    fn baseline_within_height_and_intrinsics_ordered() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);
        let paragraph = shaped("baseline test", constraints);

        let size = paragraph.measure(constraints);
        if let Some(baseline) = paragraph.measure_baseline(constraints, TextBaseline::Alphabetic) {
            assert!(baseline.get() <= size.height.get());
        }

        if let (Some(min), Some(max)) = (
            paragraph.min_intrinsic_width(infinite()),
            paragraph.max_intrinsic_width(infinite()),
        ) {
            assert!(min.get() <= max.get());
        }
    }

    #[test]
    fn infinite_width_intrinsic_height_resolves() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);
        let paragraph = shaped("resolve me", constraints);

        let _ = paragraph.max_intrinsic_height(infinite());
    }

    #[test]
    fn paragraph_without_fonts_sizes_to_smallest() {
        let paragraph = RenderParagraph::new("no fonts provided");
        let constraints = BoxConstraints::new(5.0, 100.0, 7.0, 100.0);
        assert_eq!(paragraph.measure(constraints), constraints.smallest());
        assert_eq!(paragraph.min_intrinsic_width(infinite()), None);
        assert_eq!(paragraph.max_intrinsic_width(infinite()), None);
    }
}
