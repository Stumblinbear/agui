use std::{borrow::Cow, cell::RefCell};

use parley::{FontContext, FontFamily, FontStack, Layout, LayoutContext, StyleProperty};

use crate::text::TextBrush;

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
        self.ctx.borrow_mut().collection.register_fonts(data);
    }

    /// Shapes `text` into an unbroken layout, borrowing the database and scratch for the call. A leaf
    /// operation: both borrows release on return, so a caller never juggles the two cells.
    pub fn shape(
        &self,
        text: &str,
        font_size: f32,
        brush: TextBrush,
        family: Option<&str>,
    ) -> Layout<TextBrush> {
        let mut ctx = self.ctx.borrow_mut();
        let mut scratch = self.scratch.borrow_mut();

        let mut builder = scratch.ranged_builder(&mut ctx, text, 1.0);
        builder.push_default(StyleProperty::FontSize(font_size));
        builder.push_default(StyleProperty::Brush(brush));
        if let Some(family) = family {
            builder.push_default(StyleProperty::FontStack(FontStack::Single(
                FontFamily::Named(Cow::Borrowed(family)),
            )));
        }

        builder.build(text)
    }
}
