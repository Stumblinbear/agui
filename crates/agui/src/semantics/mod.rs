//! The semantic description of a UI: a tree of nodes carrying the role, name, value, state, and
//! supported actions of what is on screen. Assistive technology reads this tree and tests query it.
//!
//! A render object records its semantics through its `build_semantics` hook; the walk assembles those
//! into a [`SemanticsTree`] snapshot, folding each render object's contribution into the nearest enclosing
//! node that accepts it. A render object that emits a node mints a [`SemanticsNodeId`] once and keeps it, so
//! the node holds a stable identity across rebuilds and reorders.
//!
//! Each node carries an [`accesskit::Node`] of accessibility properties.

mod builder;
mod config;
mod merge;
mod tree;
mod widget;

pub use accesskit::{Action, Role, Toggled};
pub use builder::SemanticsTreeBuilder;
pub use config::{SemanticsConfig, SemanticsNodeId};
pub use tree::{SemanticsNode, SemanticsTree};
pub use widget::{RenderSemanticsAnnotations, Semantics};
