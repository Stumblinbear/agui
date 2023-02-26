use agui_core::{
    unit::{Axis, ClipBehavior, Constraints, IntrinsicDimension, Point, Size, TextDirection},
    widget::{
        BuildContext, Children, ContextWidgetLayout, IntoWidget, IntrinsicSizeContext,
        LayoutContext, WidgetRef, WidgetView,
    },
};
use agui_macros::StatelessWidget;

use crate::TextBaseline;

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

#[derive(Debug, Clone, Default)]
pub struct Flexible {
    pub flex: f32,
    pub fit: FlexFit,

    pub child: WidgetRef,
}

impl<W> From<W> for Flexible
where
    W: IntoWidget,
{
    fn from(widget: W) -> Self {
        Flexible {
            child: widget.into_widget(),

            ..Flexible::default()
        }
    }
}

#[derive(StatelessWidget, Debug, Default)]
pub struct Flex {
    pub direction: Axis,

    pub main_axis_size: MainAxisSize,

    pub main_axis_alignment: MainAxisAlignment,
    pub cross_axis_alignment: CrossAxisAlignment,
    pub vertical_direction: VerticalDirection,

    pub text_direction: Option<TextDirection>,

    pub clip_behavior: ClipBehavior,

    pub children: Vec<Flexible>,
}

impl WidgetView for Flex {
    fn intrinsic_size(
        &self,
        ctx: &mut IntrinsicSizeContext<Self>,
        dimension: IntrinsicDimension,
        cross_extent: f32,
    ) -> f32 {
        if self.direction == dimension.axis() {
            // Calculate the smallest size the flex container can take while maintaining the
            // min/max-content contributions of its flex items.

            let mut total_flex = 0.0;
            let mut inflexible_space = 0.0;
            let mut max_flex_fraction_so_far = 0.0_f32;

            for (idx, child) in ctx.get_children().iter().enumerate() {
                let flex = self.children[idx].flex;

                total_flex += flex;

                let child_size = ctx.compute_intrinsic_size(*child, dimension, cross_extent);

                if flex > 0.0 {
                    let flex_fraction = child_size / flex;

                    max_flex_fraction_so_far = max_flex_fraction_so_far.max(flex_fraction);
                } else {
                    inflexible_space += child_size;
                }
            }

            max_flex_fraction_so_far * total_flex + inflexible_space
        } else {
            // The cross size is the max of the cross sizes of the children, after the flexible
            // children are fit into the available space, with the children sized using their
            // max intrinsic contributions.

            let available_space = cross_extent;
            let mut total_flex = 0.0;
            let mut inflexible_space = 0.0;
            let mut max_cross_size: f32 = 0.0;

            for (idx, child) in ctx.get_children().iter().enumerate() {
                let flex = self.children[idx].flex;

                total_flex += flex;

                let main_size: f32;
                let cross_size: f32;

                // If the flex is zero, then the child is inflexible, so we use its max intrinsic size.
                if flex == 0.0 {
                    match self.direction {
                        Axis::Horizontal => {
                            main_size = ctx.compute_intrinsic_size(
                                *child,
                                IntrinsicDimension::MaxWidth,
                                f32::INFINITY,
                            );
                            cross_size = ctx.compute_intrinsic_size(*child, dimension, main_size);
                        }
                        Axis::Vertical => {
                            main_size = ctx.compute_intrinsic_size(
                                *child,
                                IntrinsicDimension::MaxWidth,
                                f32::INFINITY,
                            );
                            cross_size = ctx.compute_intrinsic_size(*child, dimension, main_size);
                        }
                    }

                    inflexible_space += main_size;
                    max_cross_size = max_cross_size.max(cross_size);
                }
            }

            // Determine the space_per_flex by allocating the remaining available space. This may be negative
            // if we don't have enough space to accommodate all the children, so clamp it to zero.

            let space_per_flex = ((available_space - inflexible_space) / total_flex).max(0.0);

            for (idx, child) in ctx.get_children().iter().enumerate() {
                let flex = self.children[idx].flex;

                if flex > 0.0 {
                    max_cross_size = max_cross_size.max(ctx.compute_intrinsic_size(
                        *child,
                        dimension,
                        space_per_flex * flex,
                    ));
                }
            }

            max_cross_size
        }
    }

    fn layout(&self, ctx: &mut LayoutContext<Self>, constraints: Constraints) -> Size {
        let ComputedSizes {
            mut main_size,
            mut cross_size,
            allocated_size,
            child_sizes,
        } = self.compute_sizes(ctx, constraints);

        // Do we want to allow calculating layout without actually laying out the children?
        /*
           return match self.direction {
                Axis.horizontal => constraints.constrain(Size(main_size, cross_size)),
                Axis.vertical => constraints.constrain(Size(cross_size, main_size))
           };
        */

        let mut max_baseline_distance = 0.0;

        if let CrossAxisAlignment::Baseline(text_baseline) = self.cross_axis_alignment {
            let mut max_size_above_baseline = 0.0;
            let mut max_size_below_baseline = 0.0;

            for (idx, child_id) in ctx.get_children().iter().enumerate() {
                /*
                if let Some(distance) = ctx.get_distance_to_baseline(*child_id, text_baseline, only_real: true) {
                    max_baseline_distance = max_baseline_distance.max(distance);
                    max_size_above_baseline = max_size_above_baseline.max(distance);
                    max_size_below_baseline = max_size_below_baseline.max(child.size.height - distance);
                    cross_size = cross_size.max(max_size_above_baseline + max_size_below_baseline);
                }
                */
            }
        }

        let size = match self.direction {
            Axis::Horizontal => {
                let size = constraints.constrain(Size::new(main_size, cross_size));

                main_size = size.width;
                cross_size = size.height;

                size
            }

            Axis::Vertical => {
                let size = constraints.constrain(Size::new(cross_size, main_size));

                main_size = size.height;
                cross_size = size.width;

                size
            }
        };

        let actual_size_delta = main_size - allocated_size;
        let overflow = (-actual_size_delta).max(0.0);

        let remaining_space = actual_size_delta.max(0.0);

        let leading_space: f32;
        let between_space: f32;

        let flip_main_axis = self.does_start_at_top_left(self.direction);

        match self.main_axis_alignment {
            MainAxisAlignment::Start => {
                leading_space = 0.0;
                between_space = 0.0;
            }

            MainAxisAlignment::End => {
                leading_space = remaining_space;
                between_space = 0.0;
            }

            MainAxisAlignment::Center => {
                leading_space = remaining_space / 2.0;
                between_space = 0.0;
            }

            MainAxisAlignment::SpaceBetween => {
                leading_space = 0.0;

                between_space = if !ctx.get_children().is_empty() {
                    remaining_space / (ctx.get_children().len() - 1) as f32
                } else {
                    0.0
                };
            }

            MainAxisAlignment::SpaceAround => {
                between_space = if !ctx.get_children().is_empty() {
                    remaining_space / ctx.get_children().len() as f32
                } else {
                    0.0
                };

                leading_space = between_space / 2.0;
            }

            MainAxisAlignment::SpaceEvenly => {
                between_space = if !ctx.get_children().is_empty() {
                    remaining_space / (ctx.get_children().len() + 1) as f32
                } else {
                    0.0
                };

                leading_space = between_space;
            }
        }

        let mut child_main_position = if flip_main_axis {
            main_size - leading_space
        } else {
            leading_space
        };

        for (idx, child_size) in child_sizes.iter().enumerate() {
            let cross_axis = self.direction.opposite();

            let child_cross_position = match self.cross_axis_alignment {
                CrossAxisAlignment::Start | CrossAxisAlignment::End => {
                    if self.does_start_at_top_left(cross_axis)
                        == (self.cross_axis_alignment == CrossAxisAlignment::Start)
                    {
                        0.0
                    } else {
                        cross_size - child_size.get_extent(cross_axis)
                    }
                }

                CrossAxisAlignment::Center => {
                    (cross_size - child_size.get_extent(cross_axis)) / 2.0
                }

                CrossAxisAlignment::Stretch => 0.0,

                CrossAxisAlignment::Baseline(_) => {
                    // if self.direction == Axis::Horizontal {
                    //     let distance = ctx.get_distance_to_baseline(*child_id, text_baseline, onlyReal: true);

                    //     if let Some(distance) = distance {
                    //         max_baseline_distance - distance
                    //     } else {
                    //         0.0
                    //     }
                    // }

                    0.0
                }
            };

            if flip_main_axis {
                child_main_position -= child_size.get_extent(self.direction);
            }

            match self.direction {
                Axis::Horizontal => {
                    ctx.set_offset(idx, Point::new(child_main_position, child_cross_position));
                }

                Axis::Vertical => {
                    ctx.set_offset(idx, Point::new(child_cross_position, child_main_position));
                }
            };

            if flip_main_axis {
                child_main_position -= between_space;
            } else {
                child_main_position += child_size.get_extent(self.direction) + between_space;
            }
        }

        size
    }

    fn build(&self, _: &mut BuildContext<Self>) -> Children {
        Children::from(
            &self
                .children
                .iter()
                .map(|entry| entry.child.clone())
                .collect::<Vec<_>>(),
        )
    }
}

impl Flex {
    fn compute_sizes(
        &self,
        ctx: &mut LayoutContext<Self>,
        constraints: Constraints,
    ) -> ComputedSizes {
        let max_main_size = constraints.get_extent_max(self.direction);
        let can_flex = max_main_size < f32::INFINITY;

        let mut allocated_size: f32 = 0.0; // Sum of the sizes of the non-flexible children.
        let mut cross_size: f32 = 0.0;

        let mut total_flex = 0.0;
        let mut last_flexible_child = None;

        let children = ctx.get_children();

        let mut child_sizes = Vec::with_capacity(children.len());

        for (idx, child_id) in children.iter().enumerate() {
            let flex = self.children[idx].flex;

            if flex > 0.0 {
                total_flex += flex;
                last_flexible_child = Some(child_id);
            } else {
                let main_axis = self.direction;
                let cross_axis = main_axis.opposite();

                let inner_constraints = if self.cross_axis_alignment == CrossAxisAlignment::Stretch
                {
                    Constraints::tight_for(cross_axis, constraints.get_extent_max(cross_axis))
                } else {
                    Constraints::loose_for(cross_axis, constraints.get_extent_max(cross_axis))
                };

                let child_size = ctx.compute_layout(*child_id, inner_constraints);

                child_sizes.push(child_size);

                allocated_size += child_size.get_extent(main_axis);
                cross_size = cross_size.max(child_size.get_extent(cross_axis));
            }
        }

        // Distribute the remaining free space to flexible children.

        let free_space = (if can_flex { max_main_size } else { 0.0 } - allocated_size).max(0.0);

        let mut allocated_flex_space = 0.0;

        if total_flex > 0.0 {
            let space_per_flex = if can_flex {
                free_space / total_flex
            } else {
                f32::NAN
            };

            for (idx, child_id) in ctx.get_children().iter().enumerate() {
                let flex = self.children[idx].flex;
                let fit = self.children[idx].fit;

                if flex > 0.0 {
                    let max_child_extent = if can_flex {
                        if Some(child_id) == last_flexible_child {
                            free_space - allocated_flex_space
                        } else {
                            space_per_flex * flex
                        }
                    } else {
                        f32::INFINITY
                    };

                    let min_child_extent: f32 = match fit {
                        FlexFit::Tight => {
                            assert!(
                                max_child_extent < f32::INFINITY,
                                "Cannot have a tight-fit child in a flexible widget with infinite size."
                            );

                            max_child_extent
                        }
                        FlexFit::Loose => 0.0,
                    };

                    let inner_constraints =
                        if self.cross_axis_alignment == CrossAxisAlignment::Stretch {
                            match self.direction {
                                Axis::Horizontal => Constraints {
                                    min_width: min_child_extent,
                                    max_width: max_child_extent,
                                    min_height: constraints.max_height,
                                    max_height: constraints.max_height,
                                },
                                Axis::Vertical => Constraints {
                                    min_width: constraints.max_width,
                                    max_width: constraints.max_width,
                                    min_height: min_child_extent,
                                    max_height: max_child_extent,
                                },
                            }
                        } else {
                            match self.direction {
                                Axis::Horizontal => Constraints {
                                    min_width: min_child_extent,
                                    max_width: max_child_extent,
                                    max_height: constraints.max_height,
                                    ..Default::default()
                                },
                                Axis::Vertical => Constraints {
                                    max_width: constraints.max_width,
                                    min_height: min_child_extent,
                                    max_height: max_child_extent,
                                    ..Default::default()
                                },
                            }
                        };

                    let child_size = ctx.compute_layout(*child_id, inner_constraints);
                    let child_main_size = child_size.get_extent(self.direction);

                    assert!(
                        child_main_size <= max_child_extent,
                        "Child size exceeds the flex widget's maximum allowed size."
                    );

                    allocated_size += child_main_size;
                    allocated_flex_space += max_child_extent;
                    cross_size = cross_size.max(child_size.get_extent(self.direction.opposite()));
                }
            }
        }

        let ideal_size = if can_flex && self.main_axis_size == MainAxisSize::Max {
            max_main_size
        } else {
            allocated_size
        };

        ComputedSizes {
            main_size: ideal_size,
            cross_size,

            allocated_size,

            child_sizes,
        }
    }

    fn does_start_at_top_left(&self, direction: Axis) -> bool {
        match direction {
            Axis::Horizontal => match self.text_direction {
                Some(TextDirection::LeftToRight) | None => true,
                Some(TextDirection::RightToLeft) => false,
            },

            Axis::Vertical => match self.vertical_direction {
                VerticalDirection::Down => true,
                VerticalDirection::Up => false,
            },
        }
    }
}

struct ComputedSizes {
    main_size: f32,
    cross_size: f32,

    allocated_size: f32,

    child_sizes: Vec<Size>,
}
