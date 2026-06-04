use std::rc::Rc;

use fnv::FnvHashMap;
use peniko::kurbo::Point;

use crate::{hit_test::HitTestResult, offset::Offset};

/// Identifies one pointer (a finger, mouse, or stylus) across the events of a single interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PointerId(pub u64);

/// A single pointer event.
///
/// As it enters the dispatcher `position` is in the root coordinate space; as it is delivered to a
/// handler `position` has been localized into that handler's space.
#[derive(Debug, Clone)]
pub struct PointerEvent {
    pub pointer: PointerId,
    pub position: Offset,
    pub kind: PointerEventKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PointerEventKind {
    /// The pointer made contact.
    Down,
    /// The pointer moved while in contact.
    Move,
    /// The pointer broke contact.
    Up,
    /// The interaction was cancelled without a clean release.
    Cancel,
}

/// A handler that receives a pointer's events. A render object records one when it is hit, and the
/// dispatcher delivers the pointer's events to it for the life of the interaction.
pub type PointerHandler = Rc<dyn Fn(&PointerEvent)>;

/// Routes pointer events to the handlers under them.
///
/// While a pointer is held, the path found at contact stays fixed, so a press-and-drag keeps
/// reaching the handlers it started on even as it moves off them. An unheld move is a hover, routed
/// to whatever is under the cursor at that moment.
#[derive(Default)]
pub struct PointerDispatcher {
    active: FnvHashMap<PointerId, HitTestResult>,
}

impl PointerDispatcher {
    pub fn new() -> Self {
        Self::default()
    }

    /// Routes `event` to the handlers under it, each localized into its own space.
    ///
    /// A [`Down`](PointerEventKind::Down) hit-tests through `hit_test` and stores the path. A
    /// [`Move`](PointerEventKind::Move) while that pointer is held reuses the stored path, so the
    /// gesture stays with the handlers it started on even as it moves off them; a
    /// [`Move`](PointerEventKind::Move) with nothing held is a hover, hit-tested fresh at its current
    /// position. An [`Up`](PointerEventKind::Up) or [`Cancel`](PointerEventKind::Cancel) delivers,
    /// then discards the path.
    pub fn handle(&mut self, event: &PointerEvent, hit_test: impl FnOnce(Offset) -> HitTestResult) {
        match event.kind {
            PointerEventKind::Down => {
                let result = hit_test(event.position);
                Self::deliver(&result, event);
                self.active.insert(event.pointer, result);
            }

            PointerEventKind::Move => {
                if let Some(result) = self.active.get(&event.pointer) {
                    Self::deliver(result, event);
                } else {
                    Self::deliver(&hit_test(event.position), event);
                }
            }

            PointerEventKind::Up | PointerEventKind::Cancel => {
                if let Some(result) = self.active.get(&event.pointer) {
                    Self::deliver(result, event);
                }
                self.active.remove(&event.pointer);
            }
        }
    }

    /// Delivers `event` to every handler in `result`, localized into each handler's space.
    fn deliver(result: &HitTestResult, event: &PointerEvent) {
        for entry in result.path() {
            let local = Offset::from(entry.global_transform() * Point::from(event.position));

            entry.handler()(&PointerEvent {
                pointer: event.pointer,
                position: local,
                kind: event.kind,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use crate::hit_test::{HitTest, HitTestResult};

    use super::*;

    fn down(x: f32, y: f32) -> PointerEvent {
        PointerEvent {
            pointer: PointerId(1),
            position: Offset::new(x, y),
            kind: PointerEventKind::Down,
        }
    }

    fn event(kind: PointerEventKind, x: f32, y: f32) -> PointerEvent {
        PointerEvent {
            pointer: PointerId(1),
            position: Offset::new(x, y),
            kind,
        }
    }

    #[test]
    fn a_pointer_is_hit_tested_once_and_its_path_reused() {
        let log: Rc<RefCell<Vec<(PointerEventKind, f32, f32)>>> = Rc::new(RefCell::new(Vec::new()));
        let hit_tests = Cell::new(0);

        // The child sits at (10, 20), so events localize by that offset.
        let make_result = || {
            let sink = Rc::clone(&log);
            let handler: PointerHandler = Rc::new(move |event: &PointerEvent| {
                sink.borrow_mut().push((
                    event.kind,
                    event.position.x.get(),
                    event.position.y.get(),
                ));
            });

            let mut result = HitTestResult::new();
            result.with_offset(Offset::new(10.0_f32, 20.0), Offset::ZERO, |inner, _| {
                inner.add(Rc::clone(&handler));
                HitTest::Absorb
            });
            result
        };

        let mut dispatcher = PointerDispatcher::new();

        dispatcher.handle(&down(15.0, 25.0), |_| {
            hit_tests.set(hit_tests.get() + 1);
            make_result()
        });
        dispatcher.handle(&event(PointerEventKind::Move, 18.0, 30.0), |_| {
            hit_tests.set(hit_tests.get() + 1);
            make_result()
        });
        dispatcher.handle(&event(PointerEventKind::Up, 18.0, 30.0), |_| {
            hit_tests.set(hit_tests.get() + 1);
            make_result()
        });

        assert_eq!(hit_tests.get(), 1, "only the down event hit-tests");
        assert_eq!(
            *log.borrow(),
            vec![
                (PointerEventKind::Down, 5.0, 5.0),
                (PointerEventKind::Move, 8.0, 10.0),
                (PointerEventKind::Up, 8.0, 10.0),
            ],
            "every event is delivered, localized by the down-time path"
        );
    }

    #[test]
    fn a_hover_move_hit_tests_fresh_and_fires() {
        let hits: Rc<RefCell<Vec<(f32, f32)>>> = Rc::new(RefCell::new(Vec::new()));

        let make_result = || {
            let sink = Rc::clone(&hits);
            let handler: PointerHandler = Rc::new(move |event: &PointerEvent| {
                sink.borrow_mut()
                    .push((event.position.x.get(), event.position.y.get()));
            });

            let mut result = HitTestResult::new();
            result.add(handler);
            result
        };

        let mut dispatcher = PointerDispatcher::new();
        let hit_tested = Cell::new(false);

        // A move with no held gesture is a hover: it hit-tests and delivers.
        dispatcher.handle(&event(PointerEventKind::Move, 5.0, 7.0), |_| {
            hit_tested.set(true);
            make_result()
        });

        assert!(hit_tested.get(), "a hover move hit-tests fresh");
        assert_eq!(*hits.borrow(), vec![(5.0, 7.0)], "and the handler fires");
    }

    #[test]
    fn after_up_a_move_is_a_fresh_hover() {
        let mut dispatcher = PointerDispatcher::new();
        dispatcher.handle(&down(0.0, 0.0), |_| HitTestResult::new());
        dispatcher.handle(&event(PointerEventKind::Up, 0.0, 0.0), |_| {
            HitTestResult::new()
        });

        let hit_tested = Cell::new(false);
        dispatcher.handle(&event(PointerEventKind::Move, 0.0, 0.0), |_| {
            hit_tested.set(true);
            HitTestResult::new()
        });

        assert!(
            hit_tested.get(),
            "after up the path is discarded, so a move hit-tests fresh as a hover"
        );
    }
}
