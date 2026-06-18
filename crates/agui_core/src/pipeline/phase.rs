use std::cell::Cell;

/// Which pipeline the [`PipelineOwner`](super::PipelineOwner) is flushing, in the order a frame runs
/// them: build, then layout, then compositing bits, then paint, then composite. A pipeline rejects a
/// mark made once a later phase is flushing, since the pipeline being marked already ran this frame.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub(crate) enum FramePhase {
    /// No flush is in progress, so any pipeline may be marked.
    #[default]
    Idle,
    Build,
    Layout,
    CompositingBits,
    Paint,
    Composite,
}

thread_local! {
    /// The phase flushing on this thread. Only one pipeline flushes at a time, so one thread-local
    /// tracks it without threading shared state through every pipeline.
    static FRAME_PHASE: Cell<FramePhase> = const { Cell::new(FramePhase::Idle) };
}

impl FramePhase {
    /// Asserts a pipeline whose phase is `own` may be marked at the phase now flushing.
    ///
    /// # Panics
    ///
    /// Panics if a phase later than `own` is flushing, since that already-flushed pipeline cannot act on
    /// the mark this frame.
    pub(crate) fn assert_can_mark(own: FramePhase) {
        let current = FRAME_PHASE.with(Cell::get);
        assert!(
            current <= own,
            "cannot mark {own:?} work during the {current:?} phase, which already ran this frame"
        );
    }
}

/// Sets `phase` as flushing on this thread until the returned guard drops, restoring the phase that was
/// flushing before. The restore covers a panic, so a mark made between frames never reads a stale phase.
pub(crate) fn enter_phase(phase: FramePhase) -> PhaseGuard {
    PhaseGuard(FRAME_PHASE.with(|cell| cell.replace(phase)))
}

/// Restores the [`FramePhase`] that was flushing when [`enter_phase`] was called.
pub(crate) struct PhaseGuard(FramePhase);

impl Drop for PhaseGuard {
    fn drop(&mut self) {
        FRAME_PHASE.with(|cell| cell.set(self.0));
    }
}

#[cfg(test)]
mod tests {
    use super::{FramePhase, enter_phase};

    #[test]
    fn a_pipeline_may_be_marked_at_idle_or_during_an_earlier_phase() {
        // At idle every pipeline may be marked.
        FramePhase::assert_can_mark(FramePhase::Build);
        FramePhase::assert_can_mark(FramePhase::Paint);

        let _phase = enter_phase(FramePhase::Layout);
        FramePhase::assert_can_mark(FramePhase::Layout);
        FramePhase::assert_can_mark(FramePhase::CompositingBits);
        FramePhase::assert_can_mark(FramePhase::Paint);
        FramePhase::assert_can_mark(FramePhase::Composite);
    }

    #[test]
    fn entering_a_phase_restores_the_previous_one_on_drop() {
        let _outer = enter_phase(FramePhase::Build);
        {
            let _inner = enter_phase(FramePhase::Paint);
            FramePhase::assert_can_mark(FramePhase::Paint);
        }
        // Back in the build phase, layout may be marked again.
        FramePhase::assert_can_mark(FramePhase::Layout);
    }

    #[test]
    #[should_panic(expected = "cannot mark Build work during the Layout phase")]
    fn marking_build_during_layout_panics() {
        let _phase = enter_phase(FramePhase::Layout);
        FramePhase::assert_can_mark(FramePhase::Build);
    }

    #[test]
    #[should_panic(expected = "cannot mark Layout work during the Paint phase")]
    fn marking_layout_during_paint_panics() {
        let _phase = enter_phase(FramePhase::Paint);
        FramePhase::assert_can_mark(FramePhase::Layout);
    }

    #[test]
    #[should_panic(expected = "cannot mark CompositingBits work during the Paint phase")]
    fn marking_compositing_bits_during_paint_panics() {
        let _phase = enter_phase(FramePhase::Paint);
        FramePhase::assert_can_mark(FramePhase::CompositingBits);
    }
}
