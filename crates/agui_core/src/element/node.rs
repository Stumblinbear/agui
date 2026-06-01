/// Owns a single [`Element`](crate::element::Element) in the element tree.
pub struct ElementNode<E> {
    pub element: E,
}

impl<E> ElementNode<E> {
    pub fn new(element: E) -> Self {
        Self { element }
    }
}
