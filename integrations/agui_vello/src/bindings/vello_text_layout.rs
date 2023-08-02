use std::borrow::Cow;

use agui::{
    unit::{Constraints, FontStyle, IntrinsicDimension, Size},
    widgets::primitives::text::layout_controller::TextLayoutDelegate,
};
use vello::fello::{raw::FontRef, MetadataProvider};

pub struct VelloTextLayoutDelegate;

impl TextLayoutDelegate for VelloTextLayoutDelegate {
    fn compute_intrinsic_size(
        &self,
        font_style: &FontStyle,
        text: Cow<'static, str>,
        dimension: IntrinsicDimension,
        cross_axis: f32,
    ) -> f32 {
        let default_font =
            FontRef::new(include_bytes!("../../examples/fonts/DejaVuSans.ttf")).unwrap();

        let fello_size = vello::fello::Size::new(font_style.size);
        let charmap = default_font.charmap();
        let metrics = default_font.metrics(fello_size, Default::default());
        let line_height = metrics.ascent - metrics.descent + metrics.leading;
        let glyph_metrics = default_font.glyph_metrics(fello_size, Default::default());

        let mut pen_x = 0f32;
        let mut pen_y = 0f32;

        match dimension {
            IntrinsicDimension::MinWidth => {
                todo!()
            }

            // The maximum intrinsic width is the width of the widest line without wrapping
            IntrinsicDimension::MaxWidth => {
                for ch in text.chars() {
                    if ch == '\n' {
                        pen_y += line_height;
                        pen_x = 0.0;
                        continue;
                    }

                    let gid = charmap.map(ch).unwrap_or_default();
                    let advance = glyph_metrics.advance_width(gid).unwrap_or_default();

                    pen_x += advance;
                }

                pen_x
            }

            // The minimum intrinsic height is the height of the text with necessary wrapping to
            // fit within the given `cross_axis` width
            IntrinsicDimension::MinHeight => {
                for ch in text.chars() {
                    if ch == '\n' {
                        pen_y += line_height;
                        pen_x = 0.0;
                        continue;
                    }

                    let gid = charmap.map(ch).unwrap_or_default();
                    let advance = glyph_metrics.advance_width(gid).unwrap_or_default();

                    // Naive wrapping (doesn't account for word boundaries)
                    if pen_x + advance > cross_axis {
                        pen_y += line_height;
                        pen_x = 0.0;
                    }

                    pen_x += advance;
                }

                pen_y + line_height
            }

            IntrinsicDimension::MaxHeight => todo!(),
        }
    }

    fn compute_layout(
        &self,
        font_style: &FontStyle,
        text: Cow<'static, str>,
        constraints: Constraints,
    ) -> Size {
        let default_font =
            FontRef::new(include_bytes!("../../examples/fonts/DejaVuSans.ttf")).unwrap();

        let fello_size = vello::fello::Size::new(font_style.size);
        let charmap = default_font.charmap();
        let metrics = default_font.metrics(fello_size, Default::default());
        let line_height = metrics.ascent - metrics.descent + metrics.leading;
        let glyph_metrics = default_font.glyph_metrics(fello_size, Default::default());

        let mut pen_x = 0f32;
        let mut pen_y = 0f32;

        for ch in text.chars() {
            if ch == '\n' {
                pen_y += line_height;
                pen_x = 0.0;
                continue;
            }

            let gid = charmap.map(ch).unwrap_or_default();
            let advance = glyph_metrics.advance_width(gid).unwrap_or_default();

            // Naive wrapping (doesn't account for word boundaries)
            if pen_x + advance > constraints.max_width() {
                pen_y += line_height;
                pen_x = 0.0;
            }

            pen_x += advance;
        }

        Size::new(pen_x, pen_y + line_height)
    }
}
