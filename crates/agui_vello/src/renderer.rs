use std::marker::PhantomData;
use std::sync::Arc;

use agui_renderer::RenderViewId;
use parking_lot::Mutex;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rustc_hash::FxHashMap;

use crate::{fonts::VelloFonts, handle::VelloHandle, render::VelloViewRendererHandle};

pub struct VelloRenderer {
    fonts: Arc<Mutex<VelloFonts>>,

    views: FxHashMap<RenderViewId, Arc<VelloViewRendererHandle>>,
}

impl VelloRenderer {
    pub fn new() -> VelloRenderer {
        Self {
            fonts: Arc::default(),

            views: FxHashMap::default(),
        }
    }

    pub fn create_renderer<T>(&self) -> Result<Arc<VelloHandle<T>>, Box<dyn std::error::Error>>
    where
        T: HasRawWindowHandle + HasRawDisplayHandle,
    {
        Ok(Arc::new(VelloHandle {
            phantom: PhantomData,

            fonts: Arc::clone(&self.fonts),
        }))
    }

    // pub fn get_fonts(&self) -> &Arc<Mutex<VelloFonts>> {
    //     &self.fonts
    // }

    // pub fn add_font(&self, font_data: Vec<u8>) -> Result<Font, ReadError> {
    //     let font_ref = FontRef::new(Box::leak(Box::new(font_data)))?;

    //     Ok(self.fonts.lock().add_font(font_ref))
    // }
}

// impl InheritedWidget for VelloPlugin {
//     fn get_child(&self) -> Widget {
//         build! {
//             <TextLayoutController> {
//                 delegate: Rc::new(VelloTextLayoutDelegate {
//                     fonts: self.fonts.clone(),
//                 }),

//                 child: Some(self.child.clone()),
//             }
//         }
//     }

//     fn should_notify(&self, _: &Self) -> bool {
//         true
//     }
// }
