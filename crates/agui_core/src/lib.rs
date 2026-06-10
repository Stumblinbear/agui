#![warn(clippy::all, clippy::pedantic, clippy::cargo)]
#![allow(clippy::cargo_common_metadata)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::return_self_not_must_use)]
#![warn(clippy::clone_on_ref_ptr)]

// #![warn(missing_docs)]

pub mod context;
pub mod diagnostics;
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
pub mod widget;

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
                AnyElement, Element, LeafElement, MultiChildElement, RoutingId, RoutingPath,
                SingleChildElement, node::ElementNode,
            },
            scheduling::TaskHandle,
            widget::{BoxedSliverWidget, BoxedWidget, Widget},
        };

        pub use super::{shared::*, unit::*};
    }

    pub mod render_object {
        pub use crate::{
            context::{LayoutCtx, MountCtx, PaintCtx},
            input::hit_test::{HitTest, HitTestResult},
            pipeline::{layout::LayoutScope, paint::PaintScope},
            render_object::{
                AnyRenderObject, MultiChildRenderObject, RenderObject, SingleChildRenderObject,
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
