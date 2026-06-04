use std::fmt;

use peniko::kurbo::{Affine, Point};

use crate::{offset::Offset, pointer::PointerHandler};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitTest {
    /// The hit test was absorbed by the render object or at least one of its descendants.
    ///
    /// This prevents render objects below this one (i.e. its ancestors) from being hit.
    Absorb,

    /// The hit test was not absorbed by the render object.
    ///
    /// This allows render objects below this one (i.e. its ancestors) to be hit.
    Pass,
}

/// How a render object answers a hit within its own bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HitTestBehavior {
    /// Counts as hit only when a descendant is hit.
    #[default]
    DeferToChild,

    /// Always hit within its bounds, and prevent anything behind it from being hit.
    Opaque,

    /// Always hit within its bounds, while still letting things behind it be hit too.
    Translucent,
}

/// A handler recorded during a hit test, with the transform that localizes a root-space position into
/// the handler's own coordinate space.
pub struct HitTestEntry {
    handler: PointerHandler,
    transform: Affine,
}

impl HitTestEntry {
    pub fn handler(&self) -> &PointerHandler {
        &self.handler
    }

    /// The transform that maps a position in the root coordinate space into this entry's local space.
    pub fn global_transform(&self) -> Affine {
        self.transform
    }
}

impl fmt::Debug for HitTestEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HitTestEntry")
            .field("transform", &self.transform)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Default)]
pub struct HitTestResult {
    path: Vec<HitTestEntry>,
    /// The accumulated root-to-local transform at each depth of the walk.
    transforms: Vec<Affine>,
}

impl HitTestResult {
    pub fn new() -> Self {
        Self::default()
    }

    /// The hit targets, ordered most-specific first: the topmost leaf under the pointer is first, and
    /// event dispatch proceeds from there outward.
    pub fn path(&self) -> &[HitTestEntry] {
        &self.path
    }

    /// The transform mapping the root coordinate space into the space currently being tested.
    fn current_transform(&self) -> Affine {
        self.transforms.last().copied().unwrap_or_default()
    }

    /// Enters a coordinate space reached from the current one by `transform`, where `transform` maps
    /// the current space into the entered one.
    pub fn push_transform(&mut self, transform: Affine) {
        self.transforms.push(transform * self.current_transform());
    }

    pub fn pop_transform(&mut self) {
        self.transforms.pop();
    }

    /// Hit-tests under a paint `transform` (one that maps the child's space into the current one),
    /// localizing `position` into the child's space. Returns [`HitTest::Pass`] without testing when
    /// the transform is not invertible, since the child then occupies no visible area.
    pub fn with_transform(
        &mut self,
        transform: Affine,
        position: Offset,
        func: impl FnOnce(&mut Self, Offset) -> HitTest,
    ) -> HitTest {
        let inverse = transform.inverse();

        if !inverse
            .as_coeffs()
            .iter()
            .all(|coefficient| coefficient.is_finite())
        {
            return HitTest::Pass;
        }

        self.with_raw_transform(inverse, position, func)
    }

    /// Hit-tests under a `transform` that already maps the current space into the child's, localizing
    /// `position` by it directly.
    pub fn with_raw_transform(
        &mut self,
        transform: Affine,
        position: Offset,
        func: impl FnOnce(&mut Self, Offset) -> HitTest,
    ) -> HitTest {
        let local = Offset::from(transform * Point::from(position));

        self.transforms.push(transform * self.current_transform());
        let result = func(self, local);
        self.transforms.pop();

        result
    }

    /// Hit-tests a child painted at `offset`, localizing `position` into the child's space.
    pub fn with_offset(
        &mut self,
        offset: Offset,
        position: Offset,
        func: impl FnOnce(&mut Self, Offset) -> HitTest,
    ) -> HitTest {
        self.with_raw_transform(Affine::translate(-offset), position, func)
    }

    /// Records `handler` as hit at the current depth, capturing the transform that localizes a
    /// root-space position into its space. Children record themselves before their ancestors, so the
    /// path stays most-specific first.
    pub fn add(&mut self, handler: PointerHandler) {
        self.path.push(HitTestEntry {
            handler,
            transform: self.current_transform(),
        });
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::{cell::Cell, rc::Rc};

    use peniko::kurbo::{Affine, Point};

    use crate::{
        offset::Offset,
        pointer::{PointerEvent, PointerEventKind, PointerHandler, PointerId},
    };

    use super::*;

    #[test]
    fn default_transform_is_identity() {
        assert_eq!(HitTestResult::new().current_transform(), Affine::IDENTITY);
    }

    #[test]
    fn push_composes_the_child_onto_the_parent() {
        let mut result = HitTestResult::new();
        let parent = Affine::translate((10.0, 0.0));
        let child = Affine::scale(2.0);

        result.push_transform(parent);
        result.push_transform(child);

        // Root-to-local applies the parent first, then the child.
        assert_eq!(result.current_transform(), child * parent);

        result.pop_transform();
        assert_eq!(result.current_transform(), parent);
    }

    #[test]
    fn with_offset_localizes_position() {
        let mut result = HitTestResult::new();

        let hit = result.with_offset(
            Offset::new(10.0, 20.0),
            Offset::new(15.0, 25.0),
            |_, local| {
                assert_eq!(local.x.get(), 5.0);
                assert_eq!(local.y.get(), 5.0);
                HitTest::Absorb
            },
        );

        assert_eq!(hit, HitTest::Absorb);
        assert_eq!(result.current_transform(), Affine::IDENTITY);
    }

    #[test]
    fn with_transform_inverts_the_paint_transform() {
        let mut result = HitTestResult::new();

        // The paint transform scales by two, so a hit at (10, 20) localizes to (5, 10).
        let hit = result.with_transform(Affine::scale(2.0), Offset::new(10.0, 20.0), |_, local| {
            assert_eq!(local.x.get(), 5.0);
            assert_eq!(local.y.get(), 10.0);
            HitTest::Absorb
        });

        assert_eq!(hit, HitTest::Absorb);
    }

    #[test]
    fn with_transform_bails_on_a_singular_matrix() {
        let mut result = HitTestResult::new();

        let hit = result.with_transform(Affine::scale(0.0), Offset::ZERO, |_, _| {
            panic!("a singular transform should not be entered");
        });

        assert_eq!(hit, HitTest::Pass);
    }

    #[test]
    fn with_raw_transform_applies_directly_and_restores_the_stack() {
        let mut result = HitTestResult::new();
        let raw = Affine::scale(2.0);

        result.with_raw_transform(raw, Offset::new(5.0, 10.0), |inner, local| {
            assert_eq!(local.x.get(), 10.0);
            assert_eq!(local.y.get(), 20.0);
            assert_eq!(inner.current_transform(), raw);
            HitTest::Pass
        });

        assert_eq!(result.current_transform(), Affine::IDENTITY);
    }

    /// `add` records a handler and captures the transform that localizes a root-space position into
    /// its own space.
    #[test]
    fn add_captures_the_localizing_transform() {
        let recorded = Rc::new(Cell::new(None));

        let handler: PointerHandler = {
            let recorded = Rc::clone(&recorded);
            Rc::new(move |event: &PointerEvent| recorded.set(Some(event.position)))
        };

        let mut result = HitTestResult::new();
        result.with_offset(Offset::new(10.0, 20.0), Offset::ZERO, |inner, _| {
            inner.add(Rc::clone(&handler));
            HitTest::Absorb
        });

        assert_eq!(result.path().len(), 1);

        // Localizing a root-space (10, 20) through the captured transform lands at the child's origin.
        let transform = result.path()[0].global_transform();
        let local = Offset::from(transform * Point::from(Offset::new(10.0, 20.0)));
        result.path()[0].handler()(&PointerEvent {
            pointer: PointerId(0),
            position: local,
            kind: PointerEventKind::Down,
        });

        let recorded = recorded.get().expect("the handler ran");
        assert_eq!(recorded.x.get(), 0.0);
        assert_eq!(recorded.y.get(), 0.0);
    }
}
