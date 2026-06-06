//! The common imports for writing a test.

pub use std::time::Duration;

pub use crate::{
    fixtures::{IntrinsicBox, TestBox},
    gesture::TestGesture,
    probe::Probe,
    sizing::BoxSizingCheck,
    tester::WidgetTester,
};

pub use agui_core::{
    geometry::{Offset, Rect, Size},
    input::{
        hit_test::HitTestBehavior,
        pointer::{PointerEvent, PointerEventKind, PointerHandler, PointerId},
    },
    paint::{command::PaintCommand, peniko::Color, scene::Scene},
    render_object::box_layout::{BoxConstraints, RenderBox},
    scheduling::Vsync,
};
