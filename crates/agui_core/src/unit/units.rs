use super::MARGIN_OF_ERROR;

#[derive(Debug, Clone, Copy)]
pub enum Units {
    Pixels(f32),
    Percentage(f32),
    Stretch(f32),
    Auto,
}

impl std::hash::Hash for Units {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Units::Pixels(val) | Units::Percentage(val) | Units::Stretch(val) => {
                ((val * (1.0 / MARGIN_OF_ERROR)) as usize).hash(state);
            }
            Units::Auto => usize::MAX.hash(state),
        }
    }
}

impl Default for Units {
    fn default() -> Self {
        Self::Auto
    }
}

impl Units {
    pub fn value_or(&self, parent_value: f32, auto: f32) -> f32 {
        match *self {
            Units::Pixels(pixels) => pixels,
            Units::Percentage(percentage) => (percentage / 100.0) * parent_value,
            Units::Stretch(_) | Units::Auto => auto,
        }
    }
}

impl From<f32> for Units {
    fn from(px: f32) -> Self {
        Self::Pixels(px)
    }
}

#[allow(clippy::cast_precision_loss)]
impl From<usize> for Units {
    fn from(px: usize) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_lossless)]
impl From<u8> for Units {
    fn from(px: u8) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_lossless)]
impl From<u16> for Units {
    fn from(px: u16) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_precision_loss)]
impl From<u32> for Units {
    fn from(px: u32) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_precision_loss)]
impl From<u64> for Units {
    fn from(px: u64) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_lossless)]
impl From<i8> for Units {
    fn from(px: i8) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_lossless)]
impl From<i16> for Units {
    fn from(px: i16) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_precision_loss)]
impl From<i32> for Units {
    fn from(px: i32) -> Self {
        Self::Pixels(px as f32)
    }
}

#[allow(clippy::cast_precision_loss)]
impl From<i64> for Units {
    fn from(px: i64) -> Self {
        Self::Pixels(px as f32)
    }
}

impl From<Units> for morphorm::Units {
    fn from(val: Units) -> Self {
        match val {
            Units::Pixels(px) => Self::Pixels(px),
            Units::Percentage(percent) => Self::Percentage(percent),
            Units::Stretch(val) => Self::Stretch(val),
            Units::Auto => Self::Auto,
        }
    }
}
