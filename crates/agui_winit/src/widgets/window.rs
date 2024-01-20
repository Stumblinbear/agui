use std::{any::TypeId, borrow::Cow, marker::PhantomData, pin::pin, rc::Rc};

use agui_core::{
    unit::{Offset, Size},
    widget::{IntoWidget, Widget},
};
use agui_elements::stateful::{
    ContextWidgetStateMut, StatefulBuildContext, StatefulWidget, WidgetState,
};
use agui_macros::{build, StatefulWidget, WidgetProps};
use agui_primitives::sized_box::SizedBox;
use agui_renderer::RenderWindow;
use futures::{future::RemoteHandle, prelude::stream::StreamExt, Future};
use winit::{
    event::WindowEvent,
    window::{Fullscreen, Theme, WindowBuilder, WindowButtons, WindowLevel},
};

use crate::{handle::WinitWindowHandle, WinitWindowManager};
use crate::{widgets::window_layout::WinitWindowLayout, CurrentWindow};

#[derive(StatefulWidget)]
pub struct WinitWindow<RendererFn>
where
    RendererFn: Fn(&winit::window::Window) -> Box<dyn Future<Output = Box<dyn RenderWindow>> + '_>
        + Send
        + Sync
        + Clone
        + 'static,
{
    #[prop(into)]
    pub attributes: Rc<WinitWindowAttributes>,

    pub renderer: RendererFn,

    pub child: Widget,
}

impl<RendererFn> StatefulWidget for WinitWindow<RendererFn>
where
    RendererFn: Fn(&winit::window::Window) -> Box<dyn Future<Output = Box<dyn RenderWindow>> + '_>
        + Send
        + Sync
        + Clone
        + 'static,
{
    type State = WinitWindowState<RendererFn>;

    fn create_state(&self) -> Self::State {
        WinitWindowState {
            phantom: PhantomData,

            attributes: self.attributes.clone(),

            window: None,
            resize_event_task: None,

            did_set_initial_visibility: false,

            window_size: Size::ZERO,
        }
    }
}

#[derive(PartialEq, Clone, WidgetProps)]
#[props(default)]
pub struct WinitWindowAttributes {
    #[prop(!default, into)]
    title: Cow<'static, str>,

    inner_size: Option<Size>,
    min_inner_size: Option<Size>,
    max_inner_size: Option<Size>,
    position: Option<Offset>,
    resizable: Option<bool>,
    enabled_buttons: Option<WindowButtons>,
    fullscreen: Option<Fullscreen>,
    maximized: Option<bool>,
    visible: Option<bool>,
    transparent: Option<bool>,
    decorations: Option<bool>,
    // window_icon: Option<Icon>,
    preferred_theme: Option<Theme>,
    resize_increments: Option<Size>,
    content_protected: Option<bool>,
    window_level: Option<WindowLevel>,
    active: Option<bool>,
}

impl WinitWindowAttributes {
    fn apply(&self, other: &Self, window: &winit::window::Window) {
        if self.title != other.title {
            window.set_title(&self.title);
        }

        if self.inner_size != other.inner_size {
            if let Some(inner_size) = self.inner_size {
                window.set_inner_size(winit::dpi::LogicalSize::new(
                    inner_size.width as f64,
                    inner_size.height as f64,
                ));
            }
        }

        if self.min_inner_size != other.min_inner_size {
            window.set_min_inner_size(self.min_inner_size.map(|min_inner_size| {
                winit::dpi::LogicalSize::new(
                    min_inner_size.width as f64,
                    min_inner_size.height as f64,
                )
            }));
        }

        if self.max_inner_size != other.max_inner_size {
            window.set_max_inner_size(self.max_inner_size.map(|max_inner_size| {
                winit::dpi::LogicalSize::new(
                    max_inner_size.width as f64,
                    max_inner_size.height as f64,
                )
            }));
        }

        if self.position != other.position {
            if let Some(position) = self.position {
                window.set_outer_position(winit::dpi::LogicalPosition::new(
                    position.x as f64,
                    position.y as f64,
                ));
            }
        }

        if self.resizable != other.resizable {
            if let Some(resizable) = self.resizable {
                window.set_resizable(resizable);
            }
        }

        if self.enabled_buttons != other.enabled_buttons {
            if let Some(enabled_buttons) = self.enabled_buttons {
                window.set_enabled_buttons(enabled_buttons);
            }
        }

        if self.fullscreen != other.fullscreen {
            window.set_fullscreen(self.fullscreen.clone());
        }

        if self.maximized != other.maximized {
            if let Some(maximized) = self.maximized {
                window.set_maximized(maximized);
            }
        }

        if self.visible != other.visible {
            if let Some(visible) = self.visible {
                window.set_visible(visible);
            }
        }

        if self.transparent != other.transparent {
            if let Some(transparent) = self.transparent {
                window.set_transparent(transparent);
            }
        }

        if self.decorations != other.decorations {
            if let Some(decorations) = self.decorations {
                window.set_decorations(decorations);
            }
        }

        // if self.window_icon != other.window_icon {
        //     if let Some(window_icon) = self.window_icon {
        //         window.set_window_icon(Some(window_icon));
        //     }
        // }

        if self.preferred_theme != other.preferred_theme {
            window.set_theme(self.preferred_theme);
        }

        if self.resize_increments != other.resize_increments {
            window.set_resize_increments(self.resize_increments.map(|resize_increments| {
                winit::dpi::LogicalSize::new(
                    resize_increments.width as f64,
                    resize_increments.height as f64,
                )
            }));
        }

        if self.content_protected != other.content_protected {
            if let Some(content_protected) = self.content_protected {
                window.set_content_protected(content_protected);
            }
        }

        if self.window_level != other.window_level {
            if let Some(window_level) = self.window_level {
                window.set_window_level(window_level);
            }
        }

        if self.active != other.active {
            if let Some(true) = self.active {
                window.focus_window();
            }
        }
    }
}

impl From<WinitWindowAttributes> for WindowBuilder {
    fn from(value: WinitWindowAttributes) -> Self {
        let mut builder = WindowBuilder::new();

        if let Some(inner_size) = value.inner_size {
            builder = builder.with_inner_size(winit::dpi::LogicalSize::new(
                inner_size.width as f64,
                inner_size.height as f64,
            ));
        }

        if let Some(min_inner_size) = value.min_inner_size {
            builder = builder.with_min_inner_size(winit::dpi::LogicalSize::new(
                min_inner_size.width as f64,
                min_inner_size.height as f64,
            ));
        }

        if let Some(max_inner_size) = value.max_inner_size {
            builder = builder.with_max_inner_size(winit::dpi::LogicalSize::new(
                max_inner_size.width as f64,
                max_inner_size.height as f64,
            ));
        }

        if let Some(position) = value.position {
            builder = builder.with_position(winit::dpi::LogicalPosition::new(
                position.x as f64,
                position.y as f64,
            ));
        }

        if let Some(resizable) = value.resizable {
            builder = builder.with_resizable(resizable);
        }

        if let Some(enabled_buttons) = value.enabled_buttons {
            builder = builder.with_enabled_buttons(enabled_buttons);
        }

        builder = builder.with_title(value.title);

        builder = builder.with_fullscreen(value.fullscreen);

        if let Some(maximized) = value.maximized {
            builder = builder.with_maximized(maximized);
        }

        if let Some(visible) = value.visible {
            builder = builder.with_visible(visible);
        }

        if let Some(transparent) = value.transparent {
            builder = builder.with_transparent(transparent);
        }

        if let Some(decorations) = value.decorations {
            builder = builder.with_decorations(decorations);
        }

        // if let Some(window_icon) = value.window_icon {
        //     builder = builder.with_window_icon(window_icon);
        // }

        builder = builder.with_theme(value.preferred_theme);

        if let Some(resize_increments) = value.resize_increments {
            builder = builder.with_resize_increments(winit::dpi::LogicalSize::new(
                resize_increments.width as f64,
                resize_increments.height as f64,
            ));
        }

        if let Some(content_protected) = value.content_protected {
            builder = builder.with_content_protected(content_protected);
        }

        if let Some(window_level) = value.window_level {
            builder = builder.with_window_level(window_level);
        }

        if let Some(active) = value.active {
            builder = builder.with_active(active);
        }

        builder
    }
}

pub struct WinitWindowState<RendererFn> {
    phantom: PhantomData<RendererFn>,

    attributes: Rc<WinitWindowAttributes>,

    window: Option<WinitWindowHandle>,
    resize_event_task: Option<RemoteHandle<()>>,

    did_set_initial_visibility: bool,

    window_size: Size,
}

impl<RendererFn> WidgetState for WinitWindowState<RendererFn>
where
    RendererFn: Fn(&winit::window::Window) -> Box<dyn Future<Output = Box<dyn RenderWindow>> + '_>
        + Send
        + Sync
        + Clone
        + 'static,
{
    type Widget = WinitWindow<RendererFn>;

    fn init_state(&mut self, ctx: &mut StatefulBuildContext<Self>) {
        // let mouse_input_event_cb = ctx.callback(
        //     |_ctx, (device_id, state, button): (DeviceId, ElementState, MouseButton)| {
        //         // println!(
        //         //     "Mouse input event: {:?} {:?} {:?}",
        //         //     device_id, state, button
        //         // );
        //     },
        // );

        let resize_event_cb = ctx.callback(|ctx, new_window_size: Size| {
            if ctx.state.window_size == new_window_size {
                return;
            }

            ctx.set_state(move |state| {
                state.window_size = new_window_size;
            });
        });

        let Some(window_manager) = ctx.find_inherited_widget::<WinitWindowManager>() else {
            return tracing::error!(
                "WinitWindowManager was not found {:?}",
                TypeId::of::<WinitWindowManager>()
            );
        };

        let on_window_created = ctx.callback(move |ctx, window: WinitWindowHandle| {
            // let mouse_input_event_cb = mouse_input_event_cb.clone();
            // let resize_event_cb = resize_event_cb.clone();

            ctx.state.resize_event_task = ctx
                .spawn_local({
                    let window = window.clone();
                    let resize_event_cb = resize_event_cb.clone();

                    async move {
                        let mut events = pin!(window.events());

                        while let Some(event) = events.next().await {
                            if let WindowEvent::Resized(size) = event.as_ref() {
                                tracing::info!("Window resized {:?}", size);

                                resize_event_cb
                                    .call(Size::new(size.width as f32, size.height as f32));
                            }
                        }
                    }
                })
                .ok();

            ctx.set_state(|state| {
                state.window.replace(window);

                // state.event_listener = Some(window.events().add_listener(
                //     move |WinitWindowEvent(ref event)| {
                //         if let WindowEvent::MouseInput {
                //             device_id,
                //             state,
                //             button,
                //             ..
                //         } = event
                //         {
                //             mouse_input_event_cb.call((*device_id, *state, *button));
                //         } else if let WindowEvent::Resized(size) = event {
                //             resize_event_cb.call(Size::new(size.width as f32, size.height as f32));
                //         }
                //     },
                // ));

                // let size = window.inner_size();

                // state
                //     .window_size
                //     .replace(Size::new(size.width as f32, size.height as f32));
            });
        });

        let attributes = self.attributes.as_ref().clone();

        if let Err(err) = window_manager.create_window(
            move || WindowBuilder::from(attributes),
            ctx.widget.renderer.clone(),
            on_window_created,
        ) {
            tracing::error!("failed to create window: {:?}", err);
        }
    }

    fn updated(&mut self, _: &mut StatefulBuildContext<Self>, old_widget: &Self::Widget) {
        // TODO: if the attributes have changed between widget build and window creation,
        // we need to make sure to apply the attributes to the window.
        if self.attributes != old_widget.attributes {
            if let Some(window) = &mut self.window {
                tracing::debug!("Updating window attributes");

                self.attributes.apply(&self.attributes, window);
            }
        }
    }

    fn build(&mut self, ctx: &mut StatefulBuildContext<Self>) -> Widget {
        if let Some(window) = &self.window {
            if !self.did_set_initial_visibility {
                self.did_set_initial_visibility = true;

                window.set_visible(self.attributes.visible.unwrap_or(true));
            }

            build! {
                <WinitWindowLayout> {
                    size: self.window_size,

                    child: <CurrentWindow> {
                        handle: window.clone(),

                        child: ctx.widget.child.clone()
                    }
                }
            }
        } else {
            SizedBox::shrink().into_widget()
        }
    }
}
