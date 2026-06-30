/// Which phase of a frame the [`PipelineOwner`](super::PipelineOwner) is running, in order: build, then
/// layout, then compositing bits, then paint, then composite. The [`RenderPipeline`] holds the current
/// phase and rejects a mark whose phase is the one now running or one already past, since that phase has run
/// this frame and cannot act on the mark.
///
/// [`RenderPipeline`]: super::render_pipeline::RenderPipeline
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default)]
pub(crate) enum FramePhase {
    /// No frame is running, so any pipeline may be marked.
    #[default]
    Idle,
    Build,
    Layout,
    CompositingBits,
    Paint,
    Composite,
}

impl FramePhase {
    /// Asserts that a pipeline whose phase is `own` may be marked before the current phase `self` runs.
    ///
    /// # Panics
    ///
    /// Panics if `own` is the running phase `self` or one before it, since that phase has run this frame and
    /// cannot act on the mark.
    pub(crate) fn assert_can_mark(self, own: FramePhase) {
        assert!(
            self < own,
            "cannot mark {own:?} work during the {self:?} phase, which already ran this frame"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::FramePhase;

    #[test]
    fn a_pipeline_may_be_marked_at_idle_or_during_an_earlier_phase() {
        // At idle every pipeline may be marked.
        FramePhase::Idle.assert_can_mark(FramePhase::Build);
        FramePhase::Idle.assert_can_mark(FramePhase::Paint);

        // During layout, only a later pipeline may still be marked.
        FramePhase::Layout.assert_can_mark(FramePhase::CompositingBits);
        FramePhase::Layout.assert_can_mark(FramePhase::Paint);
        FramePhase::Layout.assert_can_mark(FramePhase::Composite);
    }

    #[test]
    #[should_panic(expected = "cannot mark Layout work during the Layout phase")]
    fn marking_the_running_phase_panics() {
        FramePhase::Layout.assert_can_mark(FramePhase::Layout);
    }

    #[test]
    #[should_panic(expected = "cannot mark Build work during the Layout phase")]
    fn marking_build_during_layout_panics() {
        FramePhase::Layout.assert_can_mark(FramePhase::Build);
    }

    #[test]
    #[should_panic(expected = "cannot mark Layout work during the Paint phase")]
    fn marking_layout_during_paint_panics() {
        FramePhase::Paint.assert_can_mark(FramePhase::Layout);
    }

    #[test]
    #[should_panic(expected = "cannot mark CompositingBits work during the Paint phase")]
    fn marking_compositing_bits_during_paint_panics() {
        FramePhase::Paint.assert_can_mark(FramePhase::CompositingBits);
    }
}
