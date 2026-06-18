#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![warn(clippy::undocumented_unsafe_blocks)]
#![warn(clippy::clone_on_ref_ptr)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]

pub use agui_core::{diagnostics, reactor};

pub mod context;
pub mod element;
pub mod geometry;
pub mod input;
pub mod key;
pub mod paint;
pub mod pipeline;
pub mod provide;
pub mod render_object;
pub mod scheduling;
pub mod stateful;
#[cfg(test)]
pub mod test_fixtures;
pub mod test_harness;
pub mod text;
pub mod view;
pub mod widget;
pub mod window;

pub use window::WindowRenderer;

pub mod prelude {
    mod shared {
        pub use crate::{
            diagnostics::{Diagnostics, DiagnosticsNode, ProtocolTag},
            render_object::AnyRenderObject,
            widget::AsAnyWidget,
        };
    }

    mod unit {
        pub use crate::{
            geometry::{
                Alignment, Axis, AxisDirection, EdgeInsets, EdgeInsetsGeometry, Offset, Rect, Size,
            },
            input::{
                hit_test::HitTestBehavior,
                pointer::{PointerEvent, PointerEventKind, PointerId},
            },
            text::{TextBaseline, TextDirection},
        };
    }

    pub mod element {
        pub use crate::{
            context::{Dispatch, MessageCtx, UpdateCtx},
            element::{
                AnyElement, Element, LeafElement, RoutingId, RoutingPath, RoutingTarget,
                SingleChildElement, node::ElementNode,
            },
            scheduling::TaskHandle,
            widget::{ChildrenElement, Widget, WidgetSequence},
        };

        pub use super::{shared::*, unit::*};
    }

    pub mod render_object {
        pub use crate::{
            context::{LayoutCtx, MountCtx, PaintCtx},
            input::hit_test::{HitTest, HitTestResult},
            pipeline::{
                layout::{DeferredLayoutScope, LayoutScope},
                paint::{DeferredPaintScope, PaintScope},
            },
            render_object::{
                AnyRenderObject, MultiChildRenderObject, RenderChildren, RenderObject,
                SingleChildRenderObject,
                box_layout::{AnyRenderBox, BoxConstraints, RenderBox},
                node::{RelayoutRenderNode, RenderNode},
                sliver::{AnyRenderSliver, RenderSliver},
            },
            text::{
                FontStyle, FontWeight, FontWidth, Fonts, InlineSpan, LineHeight, ParagraphContent,
                RenderParagraph, TextBrush, TextSpan, TextStyle,
            },
        };

        pub use super::{shared::*, unit::*};
    }
}

#[cfg(feature = "gpu")]
mod texture;

#[cfg(feature = "gpu")]
pub use texture::TextureRenderer;

#[cfg(feature = "gpu")]
pub use wgpu;

#[cfg(all(windows, feature = "gpu"))]
pub mod dcomp;

#[cfg(feature = "cpu")]
mod buffer;

#[cfg(feature = "cpu")]
pub use buffer::BufferRenderer;
