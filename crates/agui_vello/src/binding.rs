use std::{rc::Rc, sync::Arc};

use agui_core::{
    unit::Font,
    widget::{InheritedWidget, Widget},
};
use agui_macros::{build, InheritedWidget};
use agui_primitives::text::layout_controller::TextLayoutController;
use parking_lot::Mutex;
use vello::glyph::fello::raw::{FontRef, ReadError};

use crate::{fonts::VelloFonts, text_layout::VelloTextLayoutDelegate};

#[derive(InheritedWidget)]
pub struct VelloBinding {
    fonts: Arc<Mutex<VelloFonts>>,

    #[prop(into)]
    child: Widget,
}

impl InheritedWidget for VelloBinding {
    fn get_child(&self) -> Widget {
        build! {
            <TextLayoutController> {
                delegate: Rc::new(VelloTextLayoutDelegate {
                    fonts: self.fonts.clone(),
                }),

                child: Some(self.child.clone()),
            }
        }
    }

    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}

impl VelloBinding {
    pub fn add_font(&self, font_data: Vec<u8>) -> Result<Font, ReadError> {
        let font_ref = FontRef::new(Box::leak(Box::new(font_data)))?;

        Ok(self.fonts.lock().add_font(font_ref))
    }
}
