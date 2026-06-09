use std::{cell::RefCell, sync::Arc};

use parley::{FontContext, InlineBox, InlineBoxKind, Layout, LayoutContext};
use peniko::Blob;

use crate::{
    geometry::Size,
    text::{ParagraphContent, TextBrush},
};

pub struct Fonts {
    ctx: RefCell<FontContext>,
    scratch: RefCell<LayoutContext<TextBrush>>,
}

impl Default for Fonts {
    fn default() -> Self {
        Self::new()
    }
}

impl Fonts {
    pub fn new() -> Self {
        Self {
            ctx: RefCell::new(FontContext::new()),
            scratch: RefCell::new(LayoutContext::new()),
        }
    }

    /// Registers a font blob, making its families available to shaping.
    pub fn register(&self, data: Vec<u8>) {
        self.ctx
            .borrow_mut()
            .collection
            .register_fonts(Blob::new(Arc::new(data)), None);
    }

    /// Shapes `content` into an unbroken layout, sizing the inline placeholders in order from
    /// `placeholder_sizes`.
    pub fn shape(
        &self,
        content: &ParagraphContent,
        placeholder_sizes: &[Size],
    ) -> Layout<TextBrush> {
        let mut ctx = self.ctx.borrow_mut();
        let mut scratch = self.scratch.borrow_mut();

        let mut builder = scratch.ranged_builder(&mut ctx, &content.text, 1.0, true);

        for (range, style) in &content.runs {
            style.push_into(&mut builder, range.clone());
        }

        for (id, &index) in content.placeholders.iter().enumerate() {
            let size = placeholder_sizes.get(id).copied().unwrap_or(Size::ZERO);

            builder.push_inline_box(InlineBox {
                id: id as u64,
                index,
                width: size.width.get(),
                height: size.height.get(),
                kind: InlineBoxKind::InFlow,
            });
        }

        builder.build(&content.text)
    }
}
