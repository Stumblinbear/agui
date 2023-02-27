use super::EdgeInsets;

/// Holds information about each side.
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct Bounds {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Bounds {
    pub fn normalize(&self) -> Self {
        let mut norm = Self::clone(self);

        if self.top > (1.0 - self.bottom) {
            norm.top = 1.0 - self.bottom;
            norm.bottom = 1.0 - self.top;
        }

        if self.left > (1.0 - self.right) {
            norm.right = 1.0 - self.left;
            norm.left = 1.0 - self.right;
        }

        norm
    }

    pub fn contains(&self, point: (f32, f32)) -> bool {
        (point.0 >= self.left && point.0 <= self.right)
            && (point.1 >= self.top && point.1 <= self.bottom)
    }
}

impl From<EdgeInsets> for Bounds {
    fn from(edge_insets: EdgeInsets) -> Self {
        Self {
            top: edge_insets.top,
            right: edge_insets.right,
            bottom: edge_insets.bottom,
            left: edge_insets.left,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bounds;

    #[test]
    fn normalize_bounds() {
        let bounds = Bounds {
            top: 0.1,
            right: 0.2,
            bottom: 0.2,
            left: 0.1,
        };

        let normalized = bounds.normalize();

        assert_eq!(bounds, normalized, "bounds should be equal");

        let bounds = Bounds {
            top: 0.6,
            right: 0.6,
            bottom: 0.7,
            left: 0.7,
        };

        let normalized = bounds.normalize();

        assert!(
            (normalized.top - 0.3) <= f32::EPSILON,
            "top bound should have been normalized"
        );

        assert!(
            (normalized.right - 0.3) <= f32::EPSILON,
            "right bound should have been normalized"
        );

        assert!(
            (normalized.bottom - 0.4) <= f32::EPSILON,
            "bottom bound should have been normalized"
        );

        assert!(
            (normalized.left - 0.4) <= f32::EPSILON,
            "left bound should have been normalized"
        );
    }
}
