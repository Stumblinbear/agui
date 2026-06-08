/// Owns a single [`Element`](crate::element::Element) in the element tree.
pub struct ElementNode<E> {
    pub element: E,
}

impl<E> ElementNode<E> {
    pub fn new(element: E) -> Self {
        Self { element }
    }
}

impl<E> AsRef<E> for ElementNode<E> {
    fn as_ref(&self) -> &E {
        &self.element
    }
}

impl<E> AsMut<E> for ElementNode<E> {
    fn as_mut(&mut self) -> &mut E {
        &mut self.element
    }
}
