//! The relayout and repaint boundaries of one widget tree, merged into a single pipeline the
//! [`PipelineOwner`](super::PipelineOwner) holds and drives. Each boundary is opaque, registered by id,
//! marked stale through its handle, and drained shallowest-first for the driver to recompute.
//!
//! A render object registers a boundary the first time it qualifies (a relayout boundary under tight
//! constraints, a repaint boundary at paint), keeps the handle it gets back, and marks the boundary stale
//! through it or through a [`LayoutScope`]/[`PaintScope`] handed to descendants. Dropping the handle
//! unregisters the boundary. Depth comes from the boundary's nesting (`enclosing + 1`), recorded at
//! registration, so a flush re-enters boundaries rootmost-first.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::pipeline::FramePhase;

pub use agui_core::scope::{
    LayoutBoundaryId, LayoutScope, PaintBoundaryId, PaintScope, SemanticsBoundaryId, SemanticsScope,
};

mod layout;
mod paint;
mod semantics;

pub(crate) use layout::LayoutState;
pub(crate) use paint::PaintState;
pub(crate) use semantics::SemanticsState;

pub use layout::{DeferredLayoutScope, LayoutBoundaryHandle, RelayoutHook};
pub use paint::{CompositingBitsHook, DeferredPaintScope, PaintBoundaryHandle, RepaintHook};
pub use semantics::{DeferredSemanticsScope, SemanticsBoundaryHandle, SemanticsRebuild};

/// The relayout and repaint boundaries of one widget tree, each domain its own shared channel so a flush can
/// drain one domain while a relay re-enters another. A render object reaches a channel to register and mark a
/// boundary; the driver drives a frame's layout and paint.
#[derive(Clone)]
pub struct RenderPipeline {
    scheduler: Rc<FrameScheduler>,
    layout: Rc<LayoutState>,
    paint: Rc<PaintState>,
    semantics: Rc<SemanticsState>,
}

/// The frame-level state shared across the layout and paint domains: the running phase, and whether a frame
/// has already been requested for the pending work. A mark requests a frame. A flush clears the request as it
/// begins, so the next mark requests again.
struct FrameScheduler {
    /// The phase now running, so a mark for a phase that already ran this frame is rejected.
    phase: Cell<FramePhase>,

    /// Whether a frame is already requested for the current pending work, so a second mark does not re-fire.
    notified: Cell<bool>,

    needs_frame: RefCell<Box<dyn Fn()>>,
}

impl FrameScheduler {
    /// Requests a frame for a mark just made, firing the callback once until a flush clears the request.
    fn notify(&self) {
        if self.phase.get() == FramePhase::Idle && !self.notified.replace(true) {
            (self.needs_frame.borrow())();
        }
    }
}

impl Default for RenderPipeline {
    fn default() -> Self {
        let scheduler = Rc::new(FrameScheduler {
            phase: Cell::new(FramePhase::Idle),
            notified: Cell::new(false),
            needs_frame: RefCell::new(Box::new(|| {})),
        });

        Self {
            layout: Rc::new(LayoutState::new(Rc::clone(&scheduler))),
            paint: Rc::new(PaintState::new(Rc::clone(&scheduler))),
            semantics: Rc::new(SemanticsState::new()),
            scheduler,
        }
    }
}

impl RenderPipeline {
    /// Registers `f` to fire when the pipeline goes from fully clean to having pending work, so the driver
    /// schedules a frame.
    pub fn on_needs_frame(&self, f: Box<dyn Fn()>) {
        *self.scheduler.needs_frame.borrow_mut() = f;
    }

    /// Fires the needs-frame callback directly, for work that does not go through the pipeline's own
    /// channels: a build-only dirty the owner wants to wake the driver for.
    pub fn request_frame(&self) {
        (self.scheduler.needs_frame.borrow())();
    }

    pub(crate) fn layout(&self) -> &Rc<LayoutState> {
        &self.layout
    }

    pub(crate) fn paint(&self) -> &Rc<PaintState> {
        &self.paint
    }

    pub(crate) fn semantics(&self) -> &Rc<SemanticsState> {
        &self.semantics
    }

    pub(super) fn reset_notified(&self) {
        self.scheduler.notified.set(false);
    }

    /// Sets `phase` as the frame phase now running until the returned guard drops, which restores the
    /// previous phase.
    pub(crate) fn enter_phase(&self, phase: FramePhase) -> PhaseGuard {
        let previous = self.scheduler.phase.replace(phase);

        PhaseGuard {
            scheduler: Rc::clone(&self.scheduler),
            previous,
        }
    }
}

/// Restores the [`FramePhase`] that was running when [`RenderPipeline::enter_phase`] was called.
pub(crate) struct PhaseGuard {
    scheduler: Rc<FrameScheduler>,
    previous: FramePhase,
}

impl Drop for PhaseGuard {
    fn drop(&mut self) {
        self.scheduler.phase.set(self.previous);
    }
}
