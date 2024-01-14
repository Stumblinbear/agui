#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Color {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
    pub alpha: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            red: 1.0,
            green: 1.0,
            blue: 1.0,
            alpha: 1.0,
        }
    }
}

impl Color {
    pub const fn as_rgb(self) -> (f32, f32, f32) {
        (self.red, self.green, self.blue)
    }

    pub const fn as_rgba(self) -> (f32, f32, f32, f32) {
        (self.red, self.green, self.blue, self.alpha)
    }

    pub const fn from_rgb((red, green, blue): (f32, f32, f32)) -> Color {
        Self::from_rgba((red, green, blue, 1.0))
    }

    pub const fn from_rgba((red, green, blue, alpha): (f32, f32, f32, f32)) -> Color {
        Color {
            red,
            green,
            blue,
            alpha,
        }
    }
}

impl From<u32> for Color {
    fn from(c: u32) -> Self {
        Self {
            red: ((c >> 24) & 255) as f32,
            green: ((c >> 16) & 255) as f32,
            blue: ((c >> 8) & 255) as f32,
            alpha: (c & 255) as f32,
        }
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> Self {
        let (r, g, b, a) = color.as_rgba();

        ((r * 255.0) as Self) << 24
            | ((g * 255.0) as Self) << 16
            | ((b * 255.0) as Self) << 8
            | (a * 255.0) as Self
    }
}

impl From<i32> for Color {
    fn from(c: i32) -> Self {
        (c as u32).into()
    }
}

impl From<Color> for i32 {
    fn from(color: Color) -> Self {
        Into::<u32>::into(color) as i32
    }
}

impl From<(f32, f32, f32)> for Color {
    fn from(rgb: (f32, f32, f32)) -> Self {
        Self::from_rgb(rgb)
    }
}

impl From<Color> for (f32, f32, f32) {
    fn from(color: Color) -> Self {
        (color.red, color.green, color.blue)
    }
}

impl From<[f32; 3]> for Color {
    fn from(rgb: [f32; 3]) -> Self {
        Self::from_rgb(rgb.into())
    }
}

impl From<Color> for [f32; 3] {
    fn from(color: Color) -> Self {
        [color.red, color.green, color.blue]
    }
}

impl From<(f32, f32, f32, f32)> for Color {
    fn from(rgba: (f32, f32, f32, f32)) -> Self {
        Self::from_rgba(rgba)
    }
}

impl From<Color> for (f32, f32, f32, f32) {
    fn from(color: Color) -> Self {
        (color.red, color.green, color.blue, color.alpha)
    }
}

impl From<[f32; 4]> for Color {
    fn from(rgba: [f32; 4]) -> Self {
        Self::from_rgba(rgba.into())
    }
}

impl From<Color> for [f32; 4] {
    fn from(color: Color) -> Self {
        [color.red, color.green, color.blue, color.alpha]
    }
}

impl TryFrom<&str> for Color {
    type Error = ColorParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let hex_code = if let Some(value) = value.strip_prefix("0x") {
            value
        } else if let Some(value) = value.strip_prefix('#') {
            value
        } else {
            value
        };

        if hex_code.len() == 6 {
            let c = u32::from_str_radix(hex_code, 16)
                .map_err(|_| ColorParseError::InvalidHexCode(value.into()))?;

            Ok(Self::from_rgb((
                ((c >> 16) & 255) as f32 / 255.0,
                ((c >> 8) & 255) as f32 / 255.0,
                (c & 255) as f32 / 255.0,
            )))
        } else if hex_code.len() == 8 {
            let c = u32::from_str_radix(hex_code, 16)
                .map_err(|_| ColorParseError::InvalidHexCode(value.into()))?;

            Ok(Self::from_rgba((
                ((c >> 24) & 255) as f32 / 255.0,
                ((c >> 16) & 255) as f32 / 255.0,
                ((c >> 8) & 255) as f32 / 255.0,
                (c & 255) as f32 / 255.0,
            )))
        } else {
            Err(ColorParseError::InvalidHexCode(value.into()))
        }
    }
}

impl TryFrom<String> for Color {
    type Error = ColorParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<Color> for String {
    fn from(color: Color) -> Self {
        format!("#{:X}", Into::<u32>::into(color))
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum ColorParseError {
    #[error("string `{0}` is not a valid hex color")]
    InvalidHexCode(String),
}

// fn rgb2yiq(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
//     let y = 0.30 * r + 0.59 * g + 0.11 * b;

//     (
//         y,
//         0.74 * (r - y) - 0.27 * (b - y),
//         0.48 * (r - y) + 0.41 * (b - y),
//     )
// }

// fn yiq2rgb(y: f32, i: f32, q: f32) -> (f32, f32, f32) {
//     (
//         y + 0.9468822170900693 * i + 0.6235565819861433 * q,
//         y - 0.27478764629897834 * i - 0.6356910791873801 * q,
//         y - 1.1085450346420322 * i + 1.7090069284064666 * q,
//     )
// }

// fn rgb2hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
//     let max = if r > g && r > b {
//         r
//     } else if r < g && g > b {
//         g
//     } else {
//         b
//     };

//     let min = if r < g && r < b {
//         r
//     } else if r > g && g < b {
//         g
//     } else {
//         b
//     };

//     let delta = max - min;

//     if delta.abs() < 1e-5 {
//         return (0.0, 0.0, max);
//     }

//     let mut h = if r >= max {
//         (g - b) / delta
//     } else if g >= max {
//         2.0 + (b - r) / delta
//     } else if b >= max {
//         4.0 + (r - g) / delta
//     } else {
//         0.0
//     };

//     h *= 60.0;

//     if h < 0.0 {
//         h += 360.0;
//     }

//     (h, delta / max, max)
// }

// fn hsv2rgb(mut h: f32, s: f32, v: f32) -> (f32, f32, f32) {
//     if s <= 0.0 {
//         return (v, v, v);
//     }

//     if h >= 360.0 {
//         h = 0.0;
//     }

//     h = h / 60.0;

//     let i = h.floor() as u32;
//     let ff = h - i as f32;
//     let p = v * (1.0 - s);
//     let q = v * (1.0 - (s * ff));
//     let t = v * (1.0 - (s * (1.0 - ff)));

//     match i {
//         0 => (v, t, p),
//         1 => (q, v, p),
//         2 => (p, v, t),
//         3 => (p, q, v),
//         4 => (t, p, v),
//         5 => (v, p, q),
//         _ => (v, p, q),
//     }
// }

// fn rgb2hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
//     let max = if r > g && r > b {
//         r
//     } else if r < g && g > b {
//         g
//     } else {
//         b
//     };

//     let min = if r < g && r < b {
//         r
//     } else if r > g && g < b {
//         g
//     } else {
//         b
//     };

//     let l = (max + min) / 2.0;

//     if (max - min).abs() <= 1e-5 {
//         return (0.0, 00., l);
//     }

//     let delta = max - min;

//     let mut h = if r >= max {
//         (g - b) / delta + if g < b { 6.0 } else { 0.0 }
//     } else if g >= max {
//         2.0 + (b - r) / delta
//     } else if b >= max {
//         4.0 + (r - g) / delta
//     } else {
//         0.0
//     };

//     (
//         h / 6.0,
//         if l <= 0.5 {
//             delta / (max + min)
//         } else {
//             delta / (2.0 - max - min)
//         },
//         l,
//     )
// }

// fn hue2rgb(p: f32, q: f32, mut t: f32) -> f32 {
//     if t < 0.0 {
//         t += 1.0;
//     } else if t > 1.0 {
//         t -= 1.0
//     }

//     if t < 1.0 / 6.0 {
//         p + (q - p) * 6.0 * t
//     } else if t < 1.0 / 2.0 {
//         q
//     } else if t < 2.0 / 3.0 {
//         p + (q - p) * (2.0 / 3.0 - t) * 6.0
//     } else {
//         p
//     }
// }

// fn hsl2rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
//     if s.abs() <= 1e-5 {
//         return (l, l, l);
//     }

//     let q = if l < 0.5 {
//         l * (1.0 + s)
//     } else {
//         l + s - (l * s)
//     };

//     let p = 2.0 * l - q;

//     (
//         hue2rgb(p, q, h + 1.0 / 3.0),
//         hue2rgb(p, q, h),
//         hue2rgb(p, q, h - 1.0 / 3.0),
//     )
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_parse_hex_code() {
        assert_eq!(
            Color::try_from("FF0000".to_string()),
            Ok(Color::from_rgb((1.0, 0.0, 0.0)))
        );

        assert_eq!(
            Color::try_from("#00FF00".to_string()),
            Ok(Color::from_rgb((0.0, 1.0, 0.0)))
        );

        assert_eq!(
            Color::try_from("0x0000FF".to_string()),
            Ok(Color::from_rgb((0.0, 0.0, 1.0)))
        );

        assert_eq!(
            Color::try_from("FF000080".to_string()),
            Ok(Color::from_rgba((1.0, 0.0, 0.0, 0.5019608)))
        );

        assert_eq!(
            Color::try_from("#00FF0080".to_string()),
            Ok(Color::from_rgba((0.0, 1.0, 0.0, 0.5019608)))
        );

        assert_eq!(
            Color::try_from("0x0000FF80".to_string()),
            Ok(Color::from_rgba((0.0, 0.0, 1.0, 0.5019608)))
        );
    }
}
