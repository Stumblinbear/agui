use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use typed_floats::{Positive, PositiveFinite};

use agui::{
    context::{CreateCtx, LayoutCtx, PaintCtx, UpdateCtx},
    diagnostics::{Diagnostics, DiagnosticsNode},
    element::LeafElement as LeafRenderElement,
    geometry::{Offset, Size},
    input::hit_test::{HitTest, HitTestResult},
    paint::peniko::{Color, Fill},
    pipeline::render_pipeline::{DeferredLayoutScope, DeferredPaintScope, DeferredSemanticsScope},
    render_object::{
        RenderObject,
        box_layout::{BoxConstraints, RenderBox},
    },
    semantics::SemanticsTreeBuilder,
    text::TextBaseline,
    widget::Widget,
};

/// A leaf box widget for frame tests: it sizes to the smallest size the constraints allow, recording the size
/// it was laid out at and how many times it painted, so a test can drive a real layout/paint frame and
/// observe it.
pub struct RecordingBox {
    pub laid_out: Rc<Cell<Option<Size>>>,
    pub layouts: Rc<Cell<usize>>,
    pub paints: Rc<Cell<usize>>,
    /// A handle to this render object's enclosing relayout boundary, captured each layout, so a test can mark
    /// it and drive an isolated re-lay.
    pub boundary: Rc<RefCell<Option<DeferredLayoutScope>>>,
    /// A handle to this render object's enclosing repaint boundary, captured each paint, so a test can mark it
    /// and drive an isolated repaint.
    pub paint_boundary: Rc<RefCell<Option<DeferredPaintScope>>>,
    /// How many times this render object has been walked for semantics, so a test can confirm a flush re-walks
    /// only the boundaries that changed.
    pub semantics_builds: Rc<Cell<usize>>,
    /// A handle to this render object's enclosing semantics boundary, captured at attach, so a test can mark it
    /// from outside a pass and drive an isolated re-walk.
    pub semantics_boundary: Rc<RefCell<Option<DeferredSemanticsScope>>>,
}

impl RecordingBox {
    pub fn new() -> Self {
        Self {
            laid_out: Rc::new(Cell::new(None)),
            layouts: Rc::new(Cell::new(0)),
            paints: Rc::new(Cell::new(0)),
            boundary: Rc::new(RefCell::new(None)),
            paint_boundary: Rc::new(RefCell::new(None)),
            semantics_builds: Rc::new(Cell::new(0)),
            semantics_boundary: Rc::new(RefCell::new(None)),
        }
    }
}

impl Default for RecordingBox {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for RecordingBox {
    type Element = LeafRenderElement<RenderRecordingBox>;

    type Render = RenderRecordingBox;

    fn create(self, _ctx: &mut CreateCtx) -> LeafRenderElement<RenderRecordingBox> {
        LeafRenderElement::new(RenderRecordingBox {
            laid_out: self.laid_out,
            layouts: self.layouts,
            paints: self.paints,
            boundary: self.boundary,
            paint_boundary: self.paint_boundary,
            semantics_builds: self.semantics_builds,
            semantics_boundary: self.semantics_boundary,
        })
    }

    fn update(
        self,
        _ctx: &mut UpdateCtx<'_>,
        _element: &mut LeafRenderElement<RenderRecordingBox>,
    ) {
    }
}

/// The render object of a [`RecordingBox`].
pub struct RenderRecordingBox {
    pub laid_out: Rc<Cell<Option<Size>>>,
    pub layouts: Rc<Cell<usize>>,
    pub paints: Rc<Cell<usize>>,
    pub boundary: Rc<RefCell<Option<DeferredLayoutScope>>>,
    pub paint_boundary: Rc<RefCell<Option<DeferredPaintScope>>>,
    pub semantics_builds: Rc<Cell<usize>>,
    pub semantics_boundary: Rc<RefCell<Option<DeferredSemanticsScope>>>,
}

impl RenderObject for RenderRecordingBox {
    fn attach(&mut self, ctx: &mut UpdateCtx<'_>) {
        *self.semantics_boundary.borrow_mut() = Some(ctx.deferred_semantics_scope());
    }

    fn build_semantics(&mut self, _s: &mut SemanticsTreeBuilder<'_>) {
        self.semantics_builds.set(self.semantics_builds.get() + 1);
    }

    fn describe(&self, d: &mut Diagnostics) -> DiagnosticsNode {
        d.node_for::<Self>().finish()
    }
}

impl RenderBox for RenderRecordingBox {
    fn min_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_width(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn min_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn max_intrinsic_height(&self, _: Positive<f32>) -> Option<PositiveFinite<f32>> {
        None
    }

    fn measure(&self, constraints: BoxConstraints) -> Size {
        constraints.smallest()
    }

    fn layout(&mut self, ctx: &mut LayoutCtx, constraints: BoxConstraints) -> Size {
        let size = constraints.smallest();
        self.laid_out.set(Some(size));
        self.layouts.set(self.layouts.get() + 1);
        *self.boundary.borrow_mut() = Some(ctx.deferred_layout_scope());
        size
    }

    fn measure_baseline(&self, _: BoxConstraints, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn distance_to_baseline(&mut self, _: TextBaseline) -> Option<PositiveFinite<f32>> {
        None
    }

    fn hit_test(&self, _: &mut HitTestResult, _: Offset) -> HitTest {
        HitTest::Pass
    }

    fn update_compositing_bits(&mut self) -> bool {
        false
    }

    fn paint(&mut self, ctx: &mut PaintCtx, offset: Offset) {
        self.paints.set(self.paints.get() + 1);
        *self.paint_boundary.borrow_mut() = Some(ctx.deferred_paint_scope());

        let size = self.laid_out.get().unwrap_or(Size::new(0.0, 0.0));
        let mut canvas = ctx.canvas();
        let brush = canvas.brush(Color::BLACK);
        canvas.fill(Fill::NonZero, brush, &(offset & size));
    }
}
