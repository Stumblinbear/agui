use std::marker::PhantomData;
use std::sync::{mpsc, Arc};

use agui_core::plugin::context::{PluginAfterUpdateContext, PluginBeforeUpdateContext};
use agui_core::plugin::Capabilities;
use agui_core::{plugin::Plugin, unit::Font};
use agui_renderer::RenderViewId;
use parking_lot::Mutex;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rustc_hash::FxHashMap;
use vello::glyph::fello::raw::{FontRef, ReadError};

use crate::{
    event::VelloPluginEvent, fonts::VelloFonts, handle::VelloHandle,
    render::VelloViewRendererHandle,
};

pub struct VelloPlugin {
    fonts: Arc<Mutex<VelloFonts>>,

    views: FxHashMap<RenderViewId, Arc<VelloViewRendererHandle>>,

    events_tx: mpsc::Sender<VelloPluginEvent>,
    events_rx: mpsc::Receiver<VelloPluginEvent>,
}

impl VelloPlugin {
    pub fn new() -> VelloPlugin {
        let (events_tx, events_rx) = mpsc::channel();

        Self {
            fonts: Arc::default(),

            views: FxHashMap::default(),

            events_tx,
            events_rx,
        }
    }

    pub fn create_renderer<T>(&self) -> Result<Arc<VelloHandle<T>>, Box<dyn std::error::Error>>
    where
        T: HasRawWindowHandle + HasRawDisplayHandle,
    {
        Ok(Arc::new(VelloHandle {
            phantom: PhantomData,

            fonts: Arc::clone(&self.fonts),

            events_tx: self.events_tx.clone(),
        }))
    }

    // pub fn get_fonts(&self) -> &Arc<Mutex<VelloFonts>> {
    //     &self.fonts
    // }

    pub fn add_font(&self, font_data: Vec<u8>) -> Result<Font, ReadError> {
        let font_ref = FontRef::new(Box::leak(Box::new(font_data)))?;

        Ok(self.fonts.lock().add_font(font_ref))
    }
}

impl Plugin for VelloPlugin {
    fn capabilities(&self) -> Capabilities {
        Capabilities::BEFORE_UPDATE | Capabilities::AFTER_UPDATE
    }

    fn on_before_update(&mut self, ctx: PluginBeforeUpdateContext) {
        for event in self.events_rx.try_iter() {
            match event {
                VelloPluginEvent::ViewBind {
                    render_view_id,
                    renderer,
                } => {
                    self.views.insert(render_view_id, renderer);
                }

                VelloPluginEvent::ViewUnbind { render_view_id } => {
                    self.views.remove(&render_view_id);
                }
            }
        }
    }

    fn on_after_update(&mut self, ctx: PluginAfterUpdateContext) {
        for (render_view_id, renderer) in &self.views {}
    }
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
