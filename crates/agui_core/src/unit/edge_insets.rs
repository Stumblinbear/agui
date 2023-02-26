use super::TextDirection;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Default for EdgeInsets {
    fn default() -> Self {
        Self {
            top: 0.0,
            left: 0.0,
            bottom: 0.0,
            right: 0.0,
        }
    }
}

impl EdgeInsets {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    pub fn all(value: f32) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(vertical: f32, horizontal: f32) -> Self {
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub fn horizontal(&self) -> f32 {
        self.left + self.right
    }

    pub fn vertical(&self) -> f32 {
        self.top + self.bottom
    }

    pub fn is_zero(&self) -> bool {
        self.top == 0.0 && self.right == 0.0 && self.bottom == 0.0 && self.left == 0.0
    }

    pub fn resolve(&self, text_direction: TextDirection) -> Self {
        match text_direction {
            TextDirection::LeftToRight => *self,
            TextDirection::RightToLeft => Self {
                top: self.top,
                right: self.left,
                bottom: self.bottom,
                left: self.right,
            },
        }
    }
}
