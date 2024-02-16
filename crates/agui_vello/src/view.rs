use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use agui_core::{
    engine::rendering::{strategies::RenderingTreeTextLayoutStrategy, view::View},
    render::{canvas::Canvas, RenderObjectId},
    unit::{Constraints, IntrinsicDimension, Offset, Size, TextStyle},
    util::ptr_eq::PtrEqual,
};
use agui_renderer::FrameNotifier;
use parking_lot::{Mutex, RwLock};
use vello::glyph::skrifa::MetadataProvider;

use crate::{render::VelloScene, renderer::fonts::VelloFonts};

pub struct VelloView {
    fonts: Arc<Mutex<VelloFonts>>,
    text_layout: Box<dyn RenderingTreeTextLayoutStrategy + Send>,

    scene: Arc<RwLock<VelloScene>>,

    changes: Vec<Change>,

    frame_notifier: Arc<Mutex<Option<FrameNotifier>>>,
}

#[derive(Clone)]
pub struct VelloViewHandle {
    scene: Arc<RwLock<VelloScene>>,

    frame_notifier: Arc<Mutex<Option<FrameNotifier>>>,
}

impl VelloViewHandle {
    pub(crate) fn set_frame_notifier(&self, frame_notifier: FrameNotifier) {
        self.frame_notifier.lock().replace(frame_notifier);
    }
}

impl VelloView {
    pub(crate) fn new(fonts: Arc<Mutex<VelloFonts>>) -> Self {
        Self {
            fonts: Arc::clone(&fonts),

            text_layout: Box::new(VelloTextLayout { fonts }),

            scene: Arc::default(),

            changes: Vec::default(),

            frame_notifier: Arc::default(),
        }
    }

    pub fn is_same_view(&self, other: &Self) -> bool {
        self.scene.is_exact_ptr(&other.scene)
    }

    pub(crate) fn handle(&self) -> VelloViewHandle {
        VelloViewHandle {
            scene: Arc::clone(&self.scene),

            frame_notifier: Arc::clone(&self.frame_notifier),
        }
    }
}

impl VelloViewHandle {
    pub(crate) fn with_scene<F, Ret>(&self, func: F) -> Ret
    where
        F: FnOnce(&VelloScene) -> Ret,
    {
        let scene = self.scene.read();

        func(&scene)
    }
}

impl View for VelloView {
    fn text_layout(&self) -> &dyn RenderingTreeTextLayoutStrategy {
        self.text_layout.as_ref()
    }

    fn text_layout_mut(&mut self) -> &mut dyn RenderingTreeTextLayoutStrategy {
        self.text_layout.as_mut()
    }

    fn on_attach(
        &mut self,
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    ) {
        tracing::trace!(
            "VelloView::on_attach {:?} {:?}",
            parent_render_object_id,
            render_object_id
        );

        self.changes.push(Change::Attach {
            parent_render_object_id,
            render_object_id,
        });
    }

    fn on_detach(&mut self, render_object_id: RenderObjectId) {
        tracing::trace!("VelloView::on_detach {:?}", render_object_id);

        self.changes.push(Change::Detach { render_object_id });
    }

    fn on_size_changed(&mut self, render_object_id: RenderObjectId, size: Size) {
        tracing::trace!(
            "VelloView::on_size_changed {:?} {:?}",
            render_object_id,
            size
        );

        self.changes.push(Change::SizeChanged {
            render_object_id,
            size,
        });
    }

    fn on_offset_changed(&mut self, render_object_id: RenderObjectId, offset: Offset) {
        tracing::trace!(
            "VelloView::on_offset_changed {:?} {:?}",
            render_object_id,
            offset
        );

        self.changes.push(Change::OffsetChanged {
            render_object_id,
            offset,
        });
    }

    fn on_paint(&mut self, render_object_id: RenderObjectId, canvas: Canvas) {
        tracing::trace!("VelloView::on_paint {:?} {:?}", render_object_id, canvas);

        self.changes.push(Change::Paint {
            render_object_id,
            canvas,
        });
    }

    fn on_sync(&mut self) {
        tracing::trace!("VelloView::on_sync");

        let start = Instant::now();

        // TODO: if this is locked, we should somehow check if another frame is ready and
        // skip this one
        let mut scene = self.scene.write();

        let lock_scene_end = Instant::now();

        let mut fonts = self.fonts.lock();

        for change in self.changes.drain(..) {
            match change {
                Change::Attach {
                    parent_render_object_id,
                    render_object_id,
                } => scene.attach(parent_render_object_id, render_object_id),

                Change::Detach { render_object_id } => scene.detatch(render_object_id),

                Change::SizeChanged {
                    render_object_id,
                    size,
                } => scene.set_size(render_object_id, size),

                Change::OffsetChanged {
                    render_object_id,
                    offset,
                } => scene.set_offset(render_object_id, offset),

                Change::Paint {
                    render_object_id,
                    canvas,
                } => scene.paint(&mut fonts, render_object_id, canvas),
            }
        }

        let apply_changes_end = Instant::now();

        scene.redraw();

        let redraw_end = Instant::now();

        let frame_notifier = self.frame_notifier.lock();

        if let Some(frame_notifier) = frame_notifier.as_ref() {
            frame_notifier.notify();
        } else {
            tracing::warn!("a frame was rendered, but no frame notifier was set");
        }

        let frame_notify_end = Instant::now();

        let timings = SyncTimings {
            duration: start.elapsed(),

            lock_scene: lock_scene_end - start,
            apply_changes: apply_changes_end - lock_scene_end,
            redraw: redraw_end - apply_changes_end,
            frame_notify: frame_notify_end - redraw_end,
        };

        tracing::debug!(?timings, "sync complete");
    }
}

enum Change {
    Attach {
        parent_render_object_id: Option<RenderObjectId>,
        render_object_id: RenderObjectId,
    },

    Detach {
        render_object_id: RenderObjectId,
    },

    SizeChanged {
        render_object_id: RenderObjectId,
        size: Size,
    },

    OffsetChanged {
        render_object_id: RenderObjectId,
        offset: Offset,
    },

    Paint {
        render_object_id: RenderObjectId,
        canvas: Canvas,
    },
}

#[derive(Clone)]
struct VelloTextLayout {
    fonts: Arc<Mutex<VelloFonts>>,
}

impl RenderingTreeTextLayoutStrategy for VelloTextLayout {
    fn compute_intrinsic_size(
        &self,
        text_style: &TextStyle,
        text: &str,
        dimension: IntrinsicDimension,
        cross_axis: f32,
    ) -> f32 {
        let mut fonts = self.fonts.lock();

        let font = fonts.get_or_insert(text_style.font.clone());
        let font_ref = VelloFonts::to_font_ref(font).expect("failed to get font ref");

        let axes = font_ref.axes();
        let font_size = vello::skrifa::instance::Size::new(text_style.size);
        let var_loc = axes.location(&[] as &[(&str, f32)]);
        let charmap = font_ref.charmap();
        let metrics = font_ref.metrics(font_size, &var_loc);
        let line_height = metrics.ascent - metrics.descent + metrics.leading;
        let glyph_metrics = font_ref.glyph_metrics(font_size, &var_loc);

        let mut pen_x = 0f32;
        let mut pen_y = 0f32;

        match dimension {
            IntrinsicDimension::MinWidth => {
                todo!("minimum text intrinsic width is not yet not implemented");
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

            // The height of the text necessary to fit within the given width (`cross_axis`)
            IntrinsicDimension::MinHeight | IntrinsicDimension::MaxHeight => {
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
        }
    }

    fn compute_size(
        &mut self,
        text_style: &TextStyle,
        text: &str,
        constraints: Constraints,
    ) -> Size {
        if text.is_empty() {
            return Size::ZERO;
        }

        let mut fonts = self.fonts.lock();

        let font = fonts.get_or_insert(text_style.font.clone());
        let font_ref = VelloFonts::to_font_ref(font).expect("failed to get font ref");

        let axes = font_ref.axes();
        let font_size = vello::skrifa::instance::Size::new(text_style.size);
        let var_loc = axes.location(&[] as &[(&str, f32)]);
        let charmap = font_ref.charmap();
        let metrics = font_ref.metrics(font_size, &var_loc);
        let line_height = metrics.ascent - metrics.descent + metrics.leading;
        let glyph_metrics = font_ref.glyph_metrics(font_size, &var_loc);

        let mut pen_x = 0f32;

        let mut size = Size::new(0.0, line_height);

        for ch in text.chars() {
            if ch == '\n' {
                size.height += line_height;
                pen_x = 0.0;
                continue;
            }

            let gid = charmap.map(ch).unwrap_or_default();
            let advance = glyph_metrics.advance_width(gid).unwrap_or_default();

            // Naive wrapping (doesn't account for word boundaries)
            if pen_x + advance > constraints.max_width() {
                size.height += line_height;
                pen_x = 0.0;
            }

            pen_x += advance;

            size.width = size.width.max(pen_x);
        }

        size
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct SyncTimings {
    duration: Duration,

    lock_scene: Duration,
    apply_changes: Duration,
    redraw: Duration,
    frame_notify: Duration,
}
