use crate::unit::Shape;

#[derive(Debug, Clone, PartialEq)]
pub enum Clip {
    Hard { shape: Shape },
}
