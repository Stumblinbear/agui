use agui::geometry::Offset;

pub use agui::input::pointer::{PointerDispatcher, PointerEvent, PointerEventKind, PointerId};

use crate::tester::WidgetTester;

/// A pointer held down on the tree, moved and released step by step.
///
/// [`start_gesture`](WidgetTester::start_gesture) presses the pointer and hands back one of these; call
/// [`move_by`](Self::move_by) or [`move_to`](Self::move_to) to drag, then [`up`](Self::up) to release
/// or [`cancel`](Self::cancel) to abandon. While the pointer is held it keeps reaching the handlers it
/// pressed on, even as it moves off them.
pub struct TestGesture<'a> {
    tester: &'a mut WidgetTester,
    pointer: PointerId,
    position: Offset,
}

impl<'a> TestGesture<'a> {
    pub(crate) fn new(tester: &'a mut WidgetTester, pointer: PointerId, position: Offset) -> Self {
        Self {
            tester,
            pointer,
            position,
        }
    }

    /// Moves the pointer by `delta` from its current position.
    pub fn move_by(&mut self, delta: Offset) -> &mut Self {
        self.move_to(self.position + delta)
    }

    /// Moves the pointer to `position`.
    pub fn move_to(&mut self, position: Offset) -> &mut Self {
        self.position = position;
        self.tester
            .send_pointer(self.pointer, PointerEventKind::Move, position);
        self
    }

    /// Releases the pointer.
    pub fn up(self) {
        self.tester
            .send_pointer(self.pointer, PointerEventKind::Up, self.position);
    }

    /// Abandons the gesture without a clean release.
    pub fn cancel(self) {
        self.tester
            .send_pointer(self.pointer, PointerEventKind::Cancel, self.position);
    }
}
