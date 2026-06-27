use crate::pipeline::render_pipeline::LayoutScope;
use crate::render_object::{
    RenderObject,
    box_layout::RenderBox,
    node::{MountedChild, RenderNode},
};

/// A visitor over the children of a [`RenderChildren`], by shared reference.
pub type Visitor<'a, P> = dyn FnMut(&RenderNode<dyn RenderBox, P>) + 'a;

/// A visitor over the children of a [`RenderChildren`], by mutable reference.
pub type VisitorMut<'a, P> = dyn FnMut(&mut RenderNode<dyn RenderBox, P>) + 'a;

/// A render object that drives a single child render object, held by an edge.
///
/// The render object of every single-child widget implements this so its element can give it the child to
/// drive at mount, once that child is registered and pinned. The element calls
/// [`adopt_child`](Self::adopt_child) from its own `mount`, after mounting the child.
pub trait SingleChildRenderObject: RenderObject {
    /// The child's render object type.
    type Child: ?Sized;

    /// Adopts `child` as the render object this one drives during layout and paint. The child render object is
    /// owned by the child element; this holds only the edge to it.
    fn adopt_child(&mut self, child: MountedChild<Self::Child>);
}

/// The render object of a widget whose children are a [`RenderChildren`] list, giving its element
/// access to the child render storage so it can reconcile the list in place.
pub trait MultiChildRenderObject: RenderObject {
    /// The child render storage this object holds.
    type Children: RenderChildren;

    /// The child render storage, for the element to reconcile against a new children list.
    fn children_mut(&mut self) -> &mut Self::Children;

    /// The relayout boundary this object was laid out under at its most recent layout, for the element to mark
    /// when the child list changes structurally, since a freshly grafted child has not been laid out.
    fn layout_scope(&self) -> LayoutScope;
}

/// A flattened, ordered view of the child render objects a [`MultiChildRenderObject`] holds. The
/// storage keeps the shape of the widget sequence that built it: a single [`RenderNode`], an [`Option`], a
/// [`Vec`], or a tuple of those. This trait presents the leaves it contains as one flat run, addressable by
/// index and walkable in order.
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

// A single child: the leaf of a sequence. Its edge is already `dyn RenderBox`, so the flatten needs no
// coercion — every leaf in a mixed sequence is one uniform holder type.
impl<P> RenderChildren for RenderNode<dyn RenderBox, P> {
    type ParentData = P;

    const STATIC_LEN: usize = 1;

    fn dynamic_len(&self) -> usize {
        0
    }

    fn get(&self, index: usize) -> &RenderNode<dyn RenderBox, P> {
        assert_eq!(index, 0, "child index {index} out of bounds");
        self
    }

    fn get_mut(&mut self, index: usize) -> &mut RenderNode<dyn RenderBox, P> {
        assert_eq!(index, 0, "child index {index} out of bounds");
        self
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

#[cfg(test)]
mod tests {
    use crate::render_object::{RenderChildren, box_layout::RenderBox, node::RenderNode};

    // A leaf edge tagged by its `parent_data`, so the flatten's order and indexing are observable without
    // wiring a real child render.
    type Leaf = RenderNode<dyn RenderBox, usize>;

    fn leaf(tag: usize) -> Leaf {
        RenderNode::new(tag)
    }

    fn tags(children: &impl RenderChildren<ParentData = usize>) -> Vec<usize> {
        let mut out = Vec::new();
        children.for_each(&mut |node| out.push(node.parent_data));
        out
    }

    #[test]
    fn leaf_is_a_single_child() {
        let c = leaf(7);
        assert_eq!(Leaf::STATIC_LEN, 1);
        assert_eq!(c.len(), 1);
        assert_eq!(c.dynamic_len(), 0);
        assert_eq!(c.get(0).parent_data, 7);
        assert_eq!(tags(&c), vec![7]);
    }

    #[test]
    fn vec_flattens_in_order() {
        let c: Vec<Leaf> = vec![leaf(1), leaf(2), leaf(3)];
        assert_eq!(<Vec<Leaf>>::STATIC_LEN, 0);
        assert_eq!(c.len(), 3);
        assert_eq!(c.dynamic_len(), 3);
        assert_eq!(c.get(1).parent_data, 2);
        assert_eq!(tags(&c), vec![1, 2, 3]);
    }

    #[test]
    fn option_contributes_its_child_only_when_present() {
        let some: Option<Leaf> = Some(leaf(9));
        assert_eq!(some.len(), 1);
        assert_eq!(tags(&some), vec![9]);

        let none: Option<Leaf> = None;
        assert_eq!(none.len(), 0);
        assert!(none.is_empty());
        assert_eq!(tags(&none), Vec::<usize>::new());
    }

    #[test]
    fn tuple_concatenates_static_and_dynamic_parts() {
        let c: (Leaf, Option<Leaf>, Vec<Leaf>) = (leaf(0), Some(leaf(1)), vec![leaf(2), leaf(3)]);
        assert_eq!(<(Leaf, Option<Leaf>, Vec<Leaf>)>::STATIC_LEN, 1);
        assert_eq!(c.len(), 4);
        assert_eq!(c.get(2).parent_data, 2);
        assert_eq!(tags(&c), vec![0, 1, 2, 3]);
    }

    #[test]
    fn tuple_skips_an_absent_option() {
        let c: (Leaf, Option<Leaf>) = (leaf(5), None);
        assert_eq!(c.len(), 1);
        assert_eq!(tags(&c), vec![5]);
    }

    #[test]
    fn for_each_mut_visits_every_child() {
        let mut c: Vec<Leaf> = vec![leaf(1), leaf(2)];
        c.for_each_mut(&mut |node| node.parent_data *= 10);
        assert_eq!(tags(&c), vec![10, 20]);
    }

    #[test]
    #[should_panic(expected = "out of bounds")]
    fn get_past_the_end_panics() {
        let c: Vec<Leaf> = vec![leaf(1)];
        let _ = c.get(1);
    }
}
