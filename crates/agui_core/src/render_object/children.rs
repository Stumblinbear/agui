use crate::render_object::{box_layout::RenderBox, node::RenderNode};

/// A visitor over the children of a [`RenderChildren`], by shared reference.
pub type Visitor<'a, P> = dyn FnMut(&RenderNode<dyn RenderBox, P>) + 'a;

/// A visitor over the children of a [`RenderChildren`], by mutable reference.
pub type VisitorMut<'a, P> = dyn FnMut(&mut RenderNode<dyn RenderBox, P>) + 'a;

/// A render object that owns a single child render object.
///
/// The render of every single-child widget implements this so its element can reach the child's
/// render to reconcile it in place.
pub trait SingleChildRenderObject {
    /// The child's render object type.
    type Child: ?Sized;

    /// Runs `f` on the child render object.
    fn with_child<R>(&self, f: impl FnOnce(&Self::Child) -> R) -> R;

    /// Runs `f` on the child render object.
    fn with_child_mut<R>(&mut self, f: impl FnOnce(&mut Self::Child) -> R) -> R;
}

/// The render object of a widget whose children are a [`RenderChildren`] list, giving its element
/// access to the child render storage so it can reconcile the list in place.
pub trait MultiChildRenderObject {
    /// The child render storage this object holds.
    type Children: RenderChildren;

    /// The child render storage, for the element to reconcile against a new children list.
    fn children_mut(&mut self) -> &mut Self::Children;
}

/// A flattened, ordered view of the child render objects a [`MultiChildRenderObject`] holds. The
/// storage keeps the shape of the widget sequence that built it — a single [`RenderNode`], an
/// [`Option`], a [`Vec`], or a tuple of those — and this trait presents the leaves it contains as one
/// flat run, addressable by index and walkable in order.
///
/// The count splits into [`STATIC_LEN`](Self::STATIC_LEN), the children a value contributes regardless
/// of its runtime contents (folded at compile time), and [`dynamic_len`](Self::dynamic_len), the
/// children from `Vec` lengths and `Option` presence. A child is handed back as a [`RenderNode`] whose
/// render object is erased to `dyn RenderBox`; the children stay typed in storage.
pub trait RenderChildren {
    /// The parent data stored on each child, such as a flex slot's offset.
    type ParentData;

    /// The number of children contributed no matter the runtime contents, folded at compile time.
    const STATIC_LEN: usize;

    /// The number of children from runtime-variable parts: `Vec` lengths and `Option` presence.
    fn dynamic_len(&self) -> usize;

    /// The total number of children.
    fn len(&self) -> usize {
        Self::STATIC_LEN + self.dynamic_len()
    }

    /// Whether there are no children.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The child at flattened `index`.
    ///
    /// # Panics
    ///
    /// Panics if `index` is not less than [`len`](Self::len).
    fn get(&self, index: usize) -> &RenderNode<dyn RenderBox, Self::ParentData>;

    /// The child at flattened `index`, mutably.
    ///
    /// # Panics
    ///
    /// Panics if `index` is not less than [`len`](Self::len).
    fn get_mut(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox, Self::ParentData>;

    /// Runs `f` on each child in order. O(n) for the whole run, unlike repeated [`get`](Self::get).
    fn for_each(&self, f: &mut Visitor<Self::ParentData>);

    /// Runs `f` on each child in order, mutably.
    fn for_each_mut(&mut self, f: &mut VisitorMut<Self::ParentData>);
}

// A single render object: the leaf of a sequence, one child.
impl<R: RenderBox, P> RenderChildren for RenderNode<R, P> {
    type ParentData = P;

    const STATIC_LEN: usize = 1;

    fn dynamic_len(&self) -> usize {
        0
    }

    fn get(&self, index: usize) -> &RenderNode<dyn RenderBox, P> {
        assert_eq!(index, 0, "child index {index} out of bounds");
        self as &RenderNode<dyn RenderBox, P>
    }

    fn get_mut(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox, P> {
        assert_eq!(index, 0, "child index {index} out of bounds");
        self as &mut RenderNode<dyn RenderBox, P>
    }

    fn for_each(&self, f: &mut Visitor<P>) {
        f(self);
    }

    fn for_each_mut(&mut self, f: &mut VisitorMut<P>) {
        f(self);
    }
}

// An optional sub-sequence: present contributes its children, absent contributes none.
impl<S: RenderChildren> RenderChildren for Option<S> {
    type ParentData = S::ParentData;

    const STATIC_LEN: usize = 0;

    fn dynamic_len(&self) -> usize {
        self.as_ref().map_or(0, S::len)
    }

    fn get(&self, index: usize) -> &RenderNode<dyn RenderBox, Self::ParentData> {
        self.as_ref()
            .expect("child index out of bounds: the optional sequence is absent")
            .get(index)
    }

    fn get_mut(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox, Self::ParentData> {
        self.as_mut()
            .expect("child index out of bounds: the optional sequence is absent")
            .get_mut(index)
    }

    fn for_each(&self, f: &mut Visitor<Self::ParentData>) {
        if let Some(inner) = self {
            inner.for_each(f);
        }
    }

    fn for_each_mut(&mut self, f: &mut VisitorMut<Self::ParentData>) {
        if let Some(inner) = self {
            inner.for_each_mut(f);
        }
    }
}

// A dynamic run of sub-sequences laid end to end.
impl<S: RenderChildren> RenderChildren for Vec<S> {
    type ParentData = S::ParentData;

    const STATIC_LEN: usize = 0;

    fn dynamic_len(&self) -> usize {
        self.iter().map(S::len).sum()
    }

    fn get(&self, mut index: usize) -> &RenderNode<dyn RenderBox, Self::ParentData> {
        for child in self {
            let len = child.len();
            if index < len {
                return child.get(index);
            }
            index -= len;
        }
        panic!("child index out of bounds");
    }

    fn get_mut(&mut self, mut index: usize) -> &mut RenderNode<dyn RenderBox, Self::ParentData> {
        for child in self {
            let len = child.len();
            if index < len {
                return child.get_mut(index);
            }
            index -= len;
        }
        panic!("child index out of bounds");
    }

    fn for_each(&self, f: &mut Visitor<Self::ParentData>) {
        for child in self {
            child.for_each(f);
        }
    }

    fn for_each_mut(&mut self, f: &mut VisitorMut<Self::ParentData>) {
        for child in self {
            child.for_each_mut(f);
        }
    }
}

macro_rules! impl_render_children_tuple {
    ($($T:ident => $i:tt),+) => {
        impl<P, $($T: RenderChildren<ParentData = P>,)+> RenderChildren for ($($T,)+) {
            type ParentData = P;

            const STATIC_LEN: usize = 0 $(+ $T::STATIC_LEN)+;

            fn dynamic_len(&self) -> usize {
                0 $(+ self.$i.dynamic_len())+
            }

            fn get(&self, index: usize) -> &RenderNode<dyn RenderBox, P> {
                let mut index = index;
                $(
                    let len = self.$i.len();
                    if index < len {
                        return self.$i.get(index);
                    }
                    index -= len;
                )+
                panic!("child index {index} out of bounds");
            }

            fn get_mut(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox, P> {
                let mut index = index;
                $(
                    let len = self.$i.len();
                    if index < len {
                        return self.$i.get_mut(index);
                    }
                    index -= len;
                )+
                panic!("child index {index} out of bounds");
            }

            fn for_each(&self, f: &mut Visitor<P>) {
                $(self.$i.for_each(f);)+
            }

            fn for_each_mut(&mut self, f: &mut VisitorMut<P>) {
                $(self.$i.for_each_mut(f);)+
            }
        }
    };
}

impl_render_children_tuple!(A => 0);
impl_render_children_tuple!(A => 0, B => 1);
impl_render_children_tuple!(A => 0, B => 1, C => 2);
impl_render_children_tuple!(A => 0, B => 1, C => 2, D => 3);
impl_render_children_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4);
impl_render_children_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5);
impl_render_children_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6);
impl_render_children_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7);
impl_render_children_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8);
impl_render_children_tuple!(A => 0, B => 1, C => 2, D => 3, E => 4, F => 5, G => 6, H => 7, I => 8, J => 9);
