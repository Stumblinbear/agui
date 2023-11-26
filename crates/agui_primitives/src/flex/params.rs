use agui_core::widget::Widget;

use crate::text::TextBaseline;

use super::Flexible;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MainAxisSize {
    /// Minimize the amount of free space along the main axis, subject to the incoming layout constraints.
    ///
    /// If the incoming layout constraints have a large enough minimum width or minimum height, there might
    /// still be a non-zero amount of free space.
    ///
    /// If the incoming layout constraints are unbounded, and any children have a non-zero [FlexParentData.flex]
    /// and a [FlexFit.tight] fit (as applied by [Expanded]), the [RenderFlex] will assert, because there would
    /// be infinite remaining free space and boxes cannot be given infinite size.
    Min,

    /// Maximize the amount of free space along the main axis, subject to the incoming layout constraints.
    ///
    /// If the incoming layout constraints have a small enough [BoxConstraints.maxWidth] or [BoxConstraints.maxHeight],
    /// there might still be no free space.
    ///
    /// If the incoming layout constraints are unbounded, the widget will assert during layout, because there would be
    /// infinite remaining free space and boxes cannot be given infinite size.
    #[default]
    Max,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MainAxisAlignment {
    Start,
    #[default]
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CrossAxisAlignment {
    #[default]
    Start,

    End,

    Center,

    Stretch,

    Baseline(TextBaseline),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VerticalDirection {
    Up,
    #[default]
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FlexFit {
    #[default]
    /// The child is forced to fill the available space.
    Tight,

    /// The child can be at most as large as the available space (but is allowed to be smaller).
    Loose,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlexChildParams {
    pub flex: f32,
    pub fit: FlexFit,
}

impl From<&Widget> for FlexChildParams {
    fn from(widget: &Widget) -> Self {
        if let Some(flexible) = widget.downcast::<Flexible>() {
            Self {
                flex: flexible.flex.unwrap_or(0.0),
                fit: flexible.fit.unwrap_or_default(),
            }
        } else {
            Self {
                flex: 0.0,
                fit: FlexFit::default(),
            }
        }
    }
}
