use std::collections::VecDeque;

use crate::{element::Element, view_id::ViewId};

pub struct UpdateCtx<'a> {
    pub element: &'a mut Element,

    path: &'a mut VecDeque<ViewId>,
}

impl<'a> UpdateCtx<'a> {
    pub fn new(element: &'a mut Element, path: &'a mut VecDeque<ViewId>) -> Self {
        Self { element, path }
    }

    pub fn child(&mut self, idx: u32, func: impl FnOnce(UpdateCtx)) {
        self.path.push_back(ViewId::new(idx));

        let child = self
            .element
            .children
            .get_mut(idx as usize)
            .expect("child not found");

        func(UpdateCtx {
            element: child,
            path: self.path,
        });

        self.path.pop_back();
    }
}
