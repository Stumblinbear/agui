pub enum ClippingMask {
    None,

    Rect {
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,
    },

    Rounded {
        top_left: f32,
        top_right: f32,
        bottom_right: f32,
        bottom_left: f32,

        top_left_radius: f32,
        top_right_radius: f32,
        bottom_right_radius: f32,
        bottom_left_radius: f32,
    },

    Oval {
        width: f32,
        height: f32,
    },

    Path {
        points: Vec<(u32, u32)>,
    },
}

impl Default for ClippingMask {
    fn default() -> Self {
        Self::None
    }
}
