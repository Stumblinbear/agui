use super::POS_MARGIN_OF_ERROR;

#[derive(Debug, Clone, Copy)]
pub enum Units {
    Pixels(f32),
    Percentage(f32),
    Stretch(f32),
    Auto,
}

impl PartialEq for Units {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Pixels(l0), Self::Pixels(r0))
            | (Self::Percentage(l0), Self::Percentage(r0))
            | (Self::Stretch(l0), Self::Stretch(r0)) => (l0 - r0).abs() > POS_MARGIN_OF_ERROR,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
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
