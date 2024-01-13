use agui_renderer::RenderWindow;
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use vello::{util::RenderContext, RendererOptions};

use crate::view::VelloView;

mod sealed {
    pub trait VelloWindowRendererState {}
}

impl sealed::VelloWindowRendererState for () {}

pub struct Attached<W> {
    window: W,

    render_context: RenderContext,
    renderer: vello::Renderer,
}

pub struct VelloWindowRenderer<S> {
    view: VelloView,

    state: S,
}

impl Clone for VelloWindowRenderer<()> {
    fn clone(&self) -> Self {
        Self {
            view: self.view.clone(),

            state: (),
        }
    }
}

impl VelloWindowRenderer<()> {
    pub fn new(view: &VelloView) -> Self {
        Self {
            view: view.clone(),

            state: (),
        }
    }

    pub fn attach<W>(
        &self,
        window: W,
    ) -> Result<VelloWindowRenderer<Attached<W>>, Box<dyn std::error::Error>>
    where
        W: HasRawWindowHandle + HasRawDisplayHandle,
    {
        let mut render_context = RenderContext::new()?;

        let surface =
            futures::executor::block_on(render_context.create_surface(&window, 1_u32, 1_u32))?;

        let device_handle = &render_context.devices[surface.dev_id];

        let renderer = vello::Renderer::new(
            &device_handle.device,
            &RendererOptions {
                surface_format: Some(surface.config.format),
                timestamp_period: device_handle.queue.get_timestamp_period(),
                use_cpu: false,
            },
        )?;

        Ok(VelloWindowRenderer {
            view: self.view.clone(),

            state: Attached {
                window,

                render_context,
                renderer,
            },
        })
    }
}

impl<W> RenderWindow for VelloWindowRenderer<Attached<W>>
where
    W: HasRawWindowHandle + HasRawDisplayHandle,
{
    fn render(&self) {
        tracing::debug!("VelloWindowRenderer::render");
    }
}

// pub struct VelloWindowRenderBinding {}

// impl ViewBinding for VelloWindowRenderBinding {
//     fn on_attach(
//         &self,
//         parent_render_object_id: Option<RenderObjectId>,
//         render_object_id: RenderObjectId,
//     ) {
//         println!(
//             "VelloWindowRenderBinding::on_attach {:?} {:?}",
//             parent_render_object_id, render_object_id
//         );
//     }

//     fn on_detach(&self, render_object_id: RenderObjectId) {
//         println!("VelloWindowRenderBinding::on_detach {:?}", render_object_id);
//     }

//     fn on_size_changed(&self, render_object_id: RenderObjectId, size: Size) {
//         println!(
//             "VelloWindowRenderBinding::on_size_changed {:?} {:?}",
//             render_object_id, size
//         );
//     }

//     fn on_offset_changed(&self, render_object_id: RenderObjectId, offset: Offset) {
//         println!(
//             "VelloWindowRenderBinding::on_offset_changed {:?} {:?}",
//             render_object_id, offset
//         );
//     }

//     fn on_paint(&self, render_object_id: RenderObjectId, canvas: Canvas) {
//         println!(
//             "VelloWindowRenderBinding::on_paint {:?} {:?}",
//             render_object_id, canvas
//         );
//     }
// }
