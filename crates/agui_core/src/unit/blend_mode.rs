#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BlendMode {
    /// Clears the canvas.
    Clear,

    /// Drop the destination image, only paint the source image.
    Source,

    /// Drop the source image, only paint the destination image.
    Destination,

    /// Composite the source image over the destination image.
    SourceOver,

    /// Composite the source image under the destination image.
    DestinationOver,

    /// Show the source image, but only where the two images overlap. The destination image is
    /// not rendered, it is treated merely as a mask. The color channels of the destination are
    /// ignored, only the opacity has an effect.
    SourceIn,

    /// Show the destination image, but only where the two images overlap. The source image is
    /// not rendered, it is treated merely as a mask. The color channels of the source are ignored,
    /// only the opacity has an effect.
    DestinationIn,

    /// Show the source image, but only where the two images do not overlap. The destination image
    /// is not rendered, it is treated merely as a mask. The color channels of the destination are
    /// ignored, only the opacity has an effect.
    SourceOut,

    /// Show the destination image, but only where the two images do not overlap. The source image
    /// is not rendered, it is treated merely as a mask. The color channels of the source are
    /// ignored, only the opacity has an effect.
    DestinationOut,

    /// Composite the source image over the destination image, but only where it overlaps the destination.
    SourceAlphaTop,

    /// Composite the destination image over the source image, but only where it overlaps the source.
    DestinationAlphaTop,

    /// Apply a bitwise xor operator to the source and destination images. This leaves transparency
    /// where they would overlap.
    XOr,

    /// Sum the components of the source and destination images.
    Plus,

    /// Multiply the color components of the source and destination images.
    Modulate,

    /// Multiply the inverse of the color components of the source and destination images.
    Screen,

    /// Multiply the components of the source and destination images after adjusting them to favor
    /// the destination.
    Overlay,

    /// Composite the source and destination image by choosing the lowest value from each color channel.
    Darken,

    /// Composite the source and destination image by choosing the highest value from each color channel.
    Lighten,

    /// Divide the destination by the inverse of the source.
    ColorDodge,

    /// Divide the inverse of the destination by the source, and inverse the result.
    ColorBurn,

    /// Multiply the components of the source and destination images after adjusting them to favor
    /// the source.
    HardLight,

    /// Use ColorDodge for source values below 0.5 and ColorBurn for source values above 0.5.
    SoftLight,

    /// Subtract the smaller value from the bigger value for each channel.
    Difference,

    /// Subtract double the product of the two images from the sum of the two images.
    Exclusion,

    /// Multiply the components of the source and destination images, including the alpha channel.
    Multiply,

    /// Take the hue of the source image, and the saturation and luminosity of the destination image.
    Hue,

    /// Take the saturation of the source image, and the hue and luminosity of the destination image.
    Saturation,

    /// Take the hue and saturation of the source image, and the luminosity of the destination image.
    Color,

    /// Take the luminosity of the source image, and the hue and saturation of the destination image.
    Luminosity,
}

impl Default for BlendMode {
    fn default() -> Self {
        Self::SourceOver
    }
}
