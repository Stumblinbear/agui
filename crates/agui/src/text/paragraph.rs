use std::{cell::RefCell, rc::Rc};

use parley::{Alignment, AlignmentOptions, Decoration, GlyphRun, Layout, PositionedLayoutItem};
use peniko::{
    Fill,
    kurbo::{Line, Rect, Stroke},
};
use typed_floats::{Positive, PositiveFinite};

use crate::{
    context::{LayoutCtx, PaintCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    paint::{Canvas, command::GlyphInstance},
    pipeline::render_pipeline::DeferredLayoutScope,
    render_object::{
        MultiChildRenderObject, RenderObject,
        box_layout::{BoxConstraints, RenderBox},
        node::RenderNode,
    },
    text::{Fonts, ParagraphContent, TextBaseline, TextBrush},
};

#[derive(Clone, Copy, Default)]
struct InlineChildData {
    size: Size,
    offset: Offset,
}

/// Memoized results of the non-mutating queries, so a layout pass that fires several of them in a
/// row reshapes at most once.
#[derive(Default)]
struct QueryMemo {
    /// The unbroken shaped layout for the current content and fonts, reused across queries. Only
    /// populated when there are no inline placeholders, whose sizes would otherwise key it.
    shaped: Option<Layout<TextBrush>>,
    /// The unconstrained size at a given break width, so a repeated `measure` skips the re-break.
    broken: Option<(Option<f32>, Size)>,
}

/// A box render object that shapes, sizes, and paints rich text, laying out any inline child widgets
/// in line with it.
pub struct RenderParagraph<C: ?Sized = ()> {
    content: ParagraphContent,

    fonts: Option<Rc<Fonts>>,

    children: Vec<RenderNode<C>>,
    child_data: Vec<InlineChildData>,

    layout: Option<Layout<TextBrush>>,
    dirty: bool,
    /// The width the committed layout was last broken to, so a re-layout at the same width is reused.
    broken_width: Option<f32>,
    /// The placeholder sizes the committed layout was shaped against, so a child resize re-shapes.
    placeholder_sizes: Vec<Size>,

    memo: RefCell<QueryMemo>,

    layout_scope: DeferredLayoutScope,
    // paint_scope: PaintScope, // re-add when paint-scope capture re-homes to paint time
}

impl<C: ?Sized> RenderParagraph<C> {
    pub fn new(content: impl Into<ParagraphContent>) -> Self {
        Self {
            content: content.into(),

            fonts: None,

            children: Vec::new(),
            child_data: Vec::new(),

            layout: None,
            dirty: true,
            broken_width: None,
            placeholder_sizes: Vec::new(),

            memo: RefCell::new(QueryMemo::default()),

            layout_scope: DeferredLayoutScope::detached(),
            // paint_scope: PaintScope::detached(),
        }
    }

    pub fn set_content(&mut self, content: impl Into<ParagraphContent>) {
        let content = content.into();
        if self.content != content {
            self.content = content;
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

    fn mark_needs_reshape(&mut self) {
        self.dirty = true;
        let memo = self.memo.get_mut();
        memo.shaped = None;
        memo.broken = None;
        self.layout_scope.mark_needs_layout();
    }

    /// Installs the inline child render objects, resizing the per-child layout data to match and
    /// marking the paragraph for reshape.
    pub fn set_children(&mut self, children: Vec<RenderNode<C>>) {
        self.child_data = vec![InlineChildData::default(); children.len()];
        self.children = children;
        self.mark_needs_reshape();
    }

    /// The maximum advance to break against: the finite max width, or `None` when unbounded.
    fn max_advance(constraints: BoxConstraints) -> Option<f32> {
        constraints
            .has_bounded_width()
            .then(|| constraints.max_width().get())
    }

    /// A fresh shaped layout sized to `placeholder_sizes`, reusing the memo when there are no inline
    /// placeholders. Leaves `self` untouched.
    fn shaped_for(&self, placeholder_sizes: &[Size]) -> Option<Layout<TextBrush>> {
        let fonts = self.fonts.as_ref()?;

        if self.content.placeholders.is_empty() {
            let mut memo = self.memo.borrow_mut();
            if memo.shaped.is_none() {
                memo.shaped = Some(fonts.shape(&self.content, &[]));
            }
            memo.shaped.clone()
        } else {
            Some(fonts.shape(&self.content, placeholder_sizes))
        }
    }
}

impl<C: RenderBox + ?Sized> RenderParagraph<C> {
    fn measured_placeholder_sizes(&self, constraints: BoxConstraints) -> Vec<Size> {
        let child_constraints = BoxConstraints::loose(constraints.biggest());
        self.children
            .iter()
            .map(|child| child.measure(child_constraints))
            .collect()
    }

    fn intrinsic_placeholder_sizes(&self) -> Vec<Size> {
        self.children
            .iter()
            .map(|child| {
                let width = child
                    .max_intrinsic_width(unbounded())
                    .map_or(0.0, |extent| extent.get());
                let height = child
                    .max_intrinsic_height(unbounded())
                    .map_or(0.0, |extent| extent.get());
                Size::new(width, height)
            })
            .collect()
    }
}

impl<C: RenderObject + ?Sized> RenderObject for RenderParagraph<C> {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("text", excerpt(&self.content.text))
            .finish()
    }
}

impl MultiChildRenderObject for RenderParagraph<dyn RenderBox> {
    type Children = Vec<RenderNode<dyn RenderBox>>;

    fn children_mut(&mut self) -> &mut Vec<RenderNode<dyn RenderBox>> {
        &mut self.children
    }
}

/// `text` shortened to its first 32 characters, with an ellipsis when it was cut.
fn excerpt(text: &str) -> String {
    const LIMIT: usize = 32;

    match text.char_indices().nth(LIMIT) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text.to_owned(),
    }
}

impl<C: RenderBox + ?Sized> RenderBox for RenderParagraph<C> {
    fn measure(&self, constraints: BoxConstraints) -> Size {
        if self.fonts.is_none() {
            return constraints.smallest();
        }

        let max_advance = Self::max_advance(constraints);

        if self.content.placeholders.is_empty() {
            if let Some((width, size)) = self.memo.borrow().broken
                && width == max_advance
            {
                return constraints.constrain(size);
            }

            let Some(mut layout) = self.shaped_for(&[]) else {
                return constraints.smallest();
            };
            layout.break_all_lines(max_advance);
            let size = Size::new(layout.width(), layout.height());

            self.memo.borrow_mut().broken = Some((max_advance, size));

            return constraints.constrain(size);
        }

        let sizes = self.measured_placeholder_sizes(constraints);
        let Some(mut layout) = self.shaped_for(&sizes) else {
            return constraints.smallest();
        };
        layout.break_all_lines(max_advance);

        constraints.constrain(Size::new(layout.width(), layout.height()))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = ctx.deferred_layout_scope();

        if self.fonts.is_none() {
            return constraints.smallest();
        }

        // Reconcile installs the inline children directly through `children_mut`, not `set_children`, so keep
        // the per-child layout data the placement loop indexes in step with the children.
        self.child_data
            .resize(self.children.len(), InlineChildData::default());

        // Inline children must be sized before shaping, since their dimensions feed the line breaker.
        let placeholder_sizes = self.measured_placeholder_sizes(constraints);

        let reshaped =
            self.dirty || self.layout.is_none() || self.placeholder_sizes != placeholder_sizes;

        if reshaped {
            self.layout = self
                .fonts
                .as_ref()
                .map(|fonts| fonts.shape(&self.content, &placeholder_sizes));
            self.placeholder_sizes = placeholder_sizes;
            self.dirty = false;
            self.broken_width = None;
        }

        let max_advance = Self::max_advance(constraints);

        let Some(layout) = self.layout.as_mut() else {
            return constraints.smallest();
        };

        if reshaped || self.broken_width != max_advance {
            layout.break_all_lines(max_advance);
            layout.align(Alignment::Start, AlignmentOptions::default());
        }

        let size = Size::new(layout.width(), layout.height());
        self.broken_width = max_advance;

        // `id` is the child's index, set when the box was pushed during shaping.
        let mut placements = Vec::new();
        for line in layout.lines() {
            for item in line.items() {
                if let PositionedLayoutItem::InlineBox(inline_box) = item {
                    placements.push((
                        usize::try_from(inline_box.id).expect("inline box id is too large"),
                        Size::new(inline_box.width, inline_box.height),
                        Offset::new(inline_box.x, inline_box.y),
                    ));
                }
            }
        }

        for (index, child_size, offset) in placements {
            self.children[index].layout_and_get_size(ctx, BoxConstraints::tight(child_size));
            self.child_data[index] = InlineChildData {
                size: child_size,
                offset,
            };
        }

        constraints.constrain(size)
    }

    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let layout = self.shaped_for(&self.intrinsic_placeholder_sizes())?;
        let width = measure_clone(&layout, None).width.get();
        PositiveFinite::try_from(width).ok()
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.max_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let layout = self.shaped_for(&self.intrinsic_placeholder_sizes())?;
        let max_advance = (width.get() < f32::INFINITY).then(|| width.get());
        let height = measure_clone(&layout, max_advance).height.get();
        PositiveFinite::try_from(height).ok()
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        let mut layout = self.shaped_for(&self.intrinsic_placeholder_sizes())?;
        layout.break_all_lines(Self::max_advance(constraints));
        let baseline = layout.lines().next()?.metrics().baseline;
        PositiveFinite::try_from(baseline).ok()
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        let layout = self.layout.as_ref()?;
        let baseline = layout.lines().next()?.metrics().baseline;
        PositiveFinite::try_from(baseline).ok()
    }

    fn hit_test(&self, result: &mut HitTestResult, position: Offset) -> HitTest {
        for (child, data) in self.children.iter().zip(&self.child_data).rev() {
            let local = position - data.offset;
            if !data.size.contains(local) {
                continue;
            }

            let hit = result.with_offset(data.offset, position, |result, transformed| {
                child.hit_test(result, transformed)
            });

            if hit == HitTest::Absorb {
                return HitTest::Absorb;
            }
        }

        HitTest::Pass
    }

    fn update_compositing_bits(&mut self) -> bool {
        let mut needs = false;

        for child in &mut self.children {
            needs |= child.update_compositing_bits();
        }

        needs
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        if let Some(layout) = self.layout.as_ref() {
            let mut canvas = ctx.canvas();
            canvas.with_offset(offset, |canvas| {
                for line in layout.lines() {
                    for item in line.items() {
                        if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                            paint_glyph_run(canvas, &glyph_run);
                        }
                    }
                }
            });
        }

        for (child, data) in self.children.iter_mut().zip(&self.child_data) {
            child.paint(ctx, offset + data.offset);
        }
    }
}

/// A box render object that shapes, sizes, and paints a single run of text with no inline children. It is the
/// fast path for plain text, without the inline-placeholder machinery of [`RenderParagraph`].
pub struct RenderText {
    content: ParagraphContent,
    fonts: Option<Rc<Fonts>>,

    layout: Option<Layout<TextBrush>>,
    dirty: bool,
    /// The width the committed layout was last broken to, so a re-layout at the same width is reused.
    broken_width: Option<f32>,

    memo: RefCell<QueryMemo>,

    layout_scope: DeferredLayoutScope,
}

impl RenderText {
    pub fn new(content: impl Into<ParagraphContent>) -> Self {
        Self {
            content: content.into(),
            fonts: None,

            layout: None,
            dirty: true,
            broken_width: None,

            memo: RefCell::new(QueryMemo::default()),

            layout_scope: DeferredLayoutScope::detached(),
        }
    }

    pub fn set_content(&mut self, content: impl Into<ParagraphContent>) {
        let content = content.into();
        if self.content != content {
            self.content = content;
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

    fn mark_needs_reshape(&mut self) {
        self.dirty = true;
        let memo = self.memo.get_mut();
        memo.shaped = None;
        memo.broken = None;
        self.layout_scope.mark_needs_layout();
    }

    /// A fresh shaped layout for the current content and fonts, reusing the memo. Leaves `self` untouched.
    fn shaped(&self) -> Option<Layout<TextBrush>> {
        let fonts = self.fonts.as_ref()?;
        let mut memo = self.memo.borrow_mut();
        if memo.shaped.is_none() {
            memo.shaped = Some(fonts.shape(&self.content, &[]));
        }
        memo.shaped.clone()
    }

    /// The maximum advance to break against: the finite max width, or `None` when unbounded.
    fn max_advance(constraints: BoxConstraints) -> Option<f32> {
        constraints
            .has_bounded_width()
            .then(|| constraints.max_width().get())
    }
}

impl RenderObject for RenderText {
    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>()
            .property("text", excerpt(&self.content.text))
            .finish()
    }
}

impl RenderBox for RenderText {
    fn measure(&self, constraints: BoxConstraints) -> Size {
        if self.fonts.is_none() {
            return constraints.smallest();
        }

        let max_advance = Self::max_advance(constraints);

        if let Some((width, size)) = self.memo.borrow().broken
            && width == max_advance
        {
            return constraints.constrain(size);
        }

        let Some(mut layout) = self.shaped() else {
            return constraints.smallest();
        };
        layout.break_all_lines(max_advance);
        let size = Size::new(layout.width(), layout.height());

        self.memo.borrow_mut().broken = Some((max_advance, size));

        constraints.constrain(size)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        self.layout_scope = ctx.deferred_layout_scope();

        if self.fonts.is_none() {
            return constraints.smallest();
        }

        let reshaped = self.dirty || self.layout.is_none();

        if reshaped {
            self.layout = self
                .fonts
                .as_ref()
                .map(|fonts| fonts.shape(&self.content, &[]));
            self.dirty = false;
            self.broken_width = None;
        }

        let max_advance = Self::max_advance(constraints);

        let Some(layout) = self.layout.as_mut() else {
            return constraints.smallest();
        };

        if reshaped || self.broken_width != max_advance {
            layout.break_all_lines(max_advance);
            layout.align(Alignment::Start, AlignmentOptions::default());
        }

        let size = Size::new(layout.width(), layout.height());
        self.broken_width = max_advance;

        constraints.constrain(size)
    }

    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let layout = self.shaped()?;
        let width = measure_clone(&layout, None).width.get();
        PositiveFinite::try_from(width).ok()
    }

    fn min_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        self.max_intrinsic_height(width)
    }

    fn max_intrinsic_height(&self, width: Positive<f32>) -> Option<PositiveFinite<f32>> {
        let layout = self.shaped()?;
        let max_advance = (width.get() < f32::INFINITY).then(|| width.get());
        let height = measure_clone(&layout, max_advance).height.get();
        PositiveFinite::try_from(height).ok()
    }

    fn measure_baseline(
        &self,
        constraints: BoxConstraints,
        _: TextBaseline,
    ) -> Option<PositiveFinite<f32>> {
        let mut layout = self.shaped()?;
        layout.break_all_lines(Self::max_advance(constraints));
        let baseline = layout.lines().next()?.metrics().baseline;
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

    fn update_compositing_bits(&mut self) -> bool {
        false
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        let Some(layout) = self.layout.as_ref() else {
            return;
        };

        let mut canvas = ctx.canvas();
        canvas.with_offset(offset, |canvas| {
            for line in layout.lines() {
                for item in line.items() {
                    if let PositionedLayoutItem::GlyphRun(glyph_run) = item {
                        paint_glyph_run(canvas, &glyph_run);
                    }
                }
            }
        });
    }
}

fn unbounded() -> Positive<f32> {
    Positive::try_from(f32::INFINITY).expect("infinity is a valid positive extent")
}

fn measure_clone(layout: &Layout<TextBrush>, max_advance: Option<f32>) -> Size {
    let mut clone = layout.clone();
    clone.break_all_lines(max_advance);
    Size::new(clone.width(), clone.height())
}

/// Paints one shaped run: its highlight, then its glyphs, then its decorations.
fn paint_glyph_run(canvas: &mut Canvas, glyph_run: &GlyphRun<TextBrush>) {
    let run = glyph_run.run();
    let style = glyph_run.style();
    let metrics = run.metrics();

    if let Some(background) = &style.brush.background {
        let brush = canvas.brush(background.clone());
        let left = glyph_run.offset();
        let rect = Rect::new(
            f64::from(left),
            f64::from(glyph_run.baseline() - metrics.ascent),
            f64::from(left + glyph_run.advance()),
            f64::from(glyph_run.baseline() + metrics.descent),
        );
        canvas.fill(Fill::NonZero, brush, &rect);
    }

    let fill = canvas.brush(style.brush.fill.clone());
    let glyphs = glyph_run
        .positioned_glyphs()
        .map(|glyph| GlyphInstance {
            id: glyph.id,
            x: glyph.x,
            y: glyph.y,
        })
        .collect();
    canvas.draw_glyphs(run.font(), run.font_size(), fill, glyphs);

    if let Some(decoration) = &style.underline {
        paint_decoration(
            canvas,
            glyph_run,
            decoration,
            metrics.underline_offset,
            metrics.underline_size,
        );
    }

    if let Some(decoration) = &style.strikethrough {
        paint_decoration(
            canvas,
            glyph_run,
            decoration,
            metrics.strikethrough_offset,
            metrics.strikethrough_size,
        );
    }
}

/// Strokes one decoration line across a run, falling back to the run's metrics for an unset offset
/// or thickness.
fn paint_decoration(
    canvas: &mut Canvas,
    glyph_run: &GlyphRun<TextBrush>,
    decoration: &Decoration<TextBrush>,
    metric_offset: f32,
    metric_size: f32,
) {
    let offset = decoration.offset.unwrap_or(metric_offset);
    let size = decoration.size.unwrap_or(metric_size);

    let y = glyph_run.baseline() - offset + size / 2.0;
    let left = glyph_run.offset();
    let line = Line::new(
        (f64::from(left), f64::from(y)),
        (f64::from(left + glyph_run.advance()), f64::from(y)),
    );

    let brush = canvas.brush(decoration.brush.fill.clone());
    let stroke = canvas.stroke_style(Stroke::new(f64::from(size)));
    canvas.stroke(stroke, brush, &line);
}

// These tests shape real text through the platform font backend (DirectWrite on Windows), which Miri cannot
// call, so the module is excluded under Miri.
#[cfg(test)]
#[cfg(not(miri))]
mod tests {
    use peniko::Color;

    use crate::{
        paint::{
            command::PaintCommand,
            compositing::{Compositor, LayerHandle, OffsetLayer},
            scene::Scene,
        },
        pipeline::render_pipeline::{LayoutScope, PaintScope, RenderPipeline},
        prelude::render_object::{InlineSpan, TextSpan},
        text::{Fonts, TextBaseline, TextStyle},
    };

    use super::*;

    /// Lays `paragraph` out under a detached scope, returning the size it took.
    fn layout(paragraph: &mut RenderParagraph, constraints: BoxConstraints) -> Size {
        let pipeline = RenderPipeline::default();
        paragraph.layout(
            &mut LayoutCtx::new(&pipeline, LayoutScope::detached()),
            constraints,
        )
    }

    /// Paints `paragraph` and returns the flattened scene, for inspecting the recorded commands.
    fn paint_scene(paragraph: &mut RenderParagraph) -> Scene {
        let root = LayerHandle::new(OffsetLayer::new());
        let pipeline = RenderPipeline::default();
        PaintCtx::paint(&root, &pipeline, PaintScope::detached(), |ctx| {
            paragraph.paint(ctx, Offset::ZERO);
        });
        Compositor::compose(&root).rasterize()
    }

    fn infinite() -> Positive<f32> {
        Positive::try_from(f32::INFINITY).unwrap()
    }

    fn with_fonts(content: ParagraphContent) -> RenderParagraph {
        let mut paragraph = RenderParagraph::new(content);
        paragraph.set_fonts(Some(Rc::new(Fonts::new())));
        paragraph
    }

    /// A paragraph sized through a detached layout pass.
    fn shaped(content: ParagraphContent, constraints: BoxConstraints) -> RenderParagraph {
        let mut paragraph = with_fonts(content);
        layout(&mut paragraph, constraints);
        paragraph
    }

    /// Builds content with a single styled run over the whole string.
    fn styled(text: &str, style: TextStyle) -> ParagraphContent {
        ParagraphContent {
            runs: vec![(0..text.len(), style)],
            text: text.to_owned(),
            placeholders: Vec::new(),
        }
    }

    #[test]
    fn measure_agrees_with_layout_after_shaping() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);

        let mut paragraph = with_fonts(styled("hello world", TextStyle::new().font_size(20.0)));
        let laid_out = layout(&mut paragraph, constraints);

        assert_eq!(paragraph.measure(constraints), laid_out);
    }

    #[test]
    fn measure_works_without_a_prior_layout() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);

        let mut paragraph = with_fonts(styled("no prior layout", TextStyle::new().font_size(20.0)));

        let measured = paragraph.measure(constraints);
        let laid_out = layout(&mut paragraph, constraints);
        assert_eq!(measured, laid_out);
    }

    #[test]
    fn unbounded_width_layout_agrees_with_measure() {
        let constraints = BoxConstraints::default();

        let mut paragraph = with_fonts(styled("hello world", TextStyle::new().font_size(20.0)));

        let measured = paragraph.measure(constraints);
        let laid_out = layout(&mut paragraph, constraints);

        assert_eq!(laid_out, measured);
        assert!(laid_out.height.get() > 0.0, "the text occupies a line");
    }

    #[test]
    fn larger_font_run_grows_the_paragraph() {
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);

        let small = shaped(
            styled("ABCDEF", TextStyle::new().font_size(10.0)),
            constraints,
        );
        let large = shaped(
            styled("ABCDEF", TextStyle::new().font_size(40.0)),
            constraints,
        );

        assert!(large.measure(constraints).width > small.measure(constraints).width);
        assert!(large.measure(constraints).height > small.measure(constraints).height);
    }

    #[test]
    fn two_runs_are_wider_than_either_alone() {
        let constraints = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);

        let style = TextStyle::new().font_size(20.0);
        let one = shaped(styled("hello", style.clone()), constraints);
        let two = shaped(
            ParagraphContent {
                text: "hellohello".to_owned(),
                runs: vec![(0..5, style.clone()), (5..10, style.clone())],
                placeholders: Vec::new(),
            },
            constraints,
        );

        assert!(two.measure(constraints).width > one.measure(constraints).width);
    }

    #[test]
    fn cascade_inherits_color_and_overrides_independently() {
        // A parent color with a child that sets only a background keeps the inherited color.
        let parent = TextStyle::new().color(Color::from_rgb8(10, 20, 30));
        let child = TextStyle::new().background(Color::from_rgb8(40, 50, 60));

        let span = TextSpan::new("a")
            .style(parent.clone())
            .children([InlineSpan::Text(
                TextSpan::<()>::new("b").style(child.clone()),
            )]);

        let (content, _) = span.flatten();

        let (_, second) = &content.runs[1];
        assert_eq!(second.color, parent.color);
        assert_eq!(second.background, child.background);
    }

    #[test]
    fn background_run_paints_a_fill_behind_the_glyphs() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);
        let mut paragraph = shaped(
            styled(
                "hi",
                TextStyle::new()
                    .font_size(20.0)
                    .background(Color::from_rgb8(200, 100, 50)),
            ),
            constraints,
        );

        let scene = paint_scene(&mut paragraph);

        let has_fill = scene
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::Fill { .. }));
        assert!(
            has_fill,
            "a highlighted run records a fill behind its glyphs"
        );
    }

    #[test]
    fn underline_run_paints_a_stroke() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);
        let mut paragraph = shaped(
            styled("hi", TextStyle::new().font_size(20.0).underline(true)),
            constraints,
        );

        let scene = paint_scene(&mut paragraph);

        let has_stroke = scene
            .commands()
            .iter()
            .any(|command| matches!(command, PaintCommand::Stroke { .. }));
        assert!(has_stroke, "an underlined run records a stroke");
    }

    #[test]
    fn narrower_break_does_not_grow_width() {
        let wide = BoxConstraints::new(0.0, 1000.0, 0.0, 1000.0);
        let paragraph = shaped(
            styled(
                "the quick brown fox jumps over the lazy dog",
                TextStyle::new().font_size(20.0),
            ),
            wide,
        );

        let wide_width = paragraph.measure(wide).width;
        let narrow = BoxConstraints::new(0.0, 80.0, 0.0, 1000.0);
        let narrow_width = paragraph.measure(narrow).width;

        assert!(narrow_width <= wide_width);
    }

    #[test]
    fn baseline_within_height_and_intrinsics_ordered() {
        let constraints = BoxConstraints::new(0.0, 300.0, 0.0, 300.0);
        let paragraph = shaped(
            styled("baseline test", TextStyle::new().font_size(20.0)),
            constraints,
        );

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
    fn paragraph_without_fonts_sizes_to_smallest() {
        let paragraph = RenderParagraph::<()>::new("no fonts provided");
        let constraints = BoxConstraints::new(5.0, 100.0, 7.0, 100.0);
        assert_eq!(paragraph.measure(constraints), constraints.smallest());
        assert_eq!(paragraph.min_intrinsic_width(infinite()), None);
        assert_eq!(paragraph.max_intrinsic_width(infinite()), None);
    }
}
