use std::{cell::RefCell, rc::Rc, sync::Arc};

use parley::{FontContext, InlineBox, InlineBoxKind, Layout, LayoutContext};
use peniko::Blob;

use crate::{
    geometry::Size,
    text::{ParagraphContent, TextBrush},
};

/// A shared handle to a font registry and its shaping scratch space.
///
/// Cloning shares the same registry, so fonts registered through one handle are visible through every
/// clone. Two handles compare equal only when they refer to the same registry, so a value provided
/// through this handle changes only when a different registry is provided, never when fonts are
/// registered into the existing one.
#[derive(Clone)]
pub struct Fonts(Rc<FontsInner>);

struct FontsInner {
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
        Self(Rc::new(FontsInner {
            ctx: RefCell::new(FontContext::new()),
            scratch: RefCell::new(LayoutContext::new()),
        }))
    }

    /// Registers a font blob, making its families available to shaping.
    pub fn register(&self, data: Vec<u8>) {
        self.0
            .ctx
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
        let mut ctx = self.0.ctx.borrow_mut();
        let mut scratch = self.0.scratch.borrow_mut();

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

impl PartialEq for Fonts {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
