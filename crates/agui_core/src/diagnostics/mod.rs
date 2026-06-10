use std::{
    borrow::Cow,
    fmt::{self, Display, Write},
};

/// A layout protocol's identity in a diagnostics tree.
///
/// A node is stamped with the protocol it lays out under whenever that protocol differs from its
/// parent's, marking the seam where one protocol gives way to another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolTag(&'static str);

impl ProtocolTag {
    /// The Cartesian box-layout protocol.
    pub const BOX: ProtocolTag = ProtocolTag("box");

    /// The scroll-axis sliver-layout protocol.
    pub const SLIVER: ProtocolTag = ProtocolTag("sliver");

    /// A protocol identified by `name`.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// The protocol's name as it appears in a dump.
    pub fn name(&self) -> &'static str {
        self.0
    }
}

/// The context a diagnostics description is built in, carrying state shared across one capture such as
/// the active layout protocol.
pub struct Diagnostics {
    protocol: Option<ProtocolTag>,

    pending_decorations: Vec<DiagnosticsProperty>,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self::new()
    }
}

impl Diagnostics {
    /// A fresh context for one capture.
    pub fn new() -> Self {
        Self {
            protocol: None,

            pending_decorations: Vec::new(),
        }
    }

    /// Begins decorating the child built next: the properties added here are claimed by its node.
    pub fn decorate(&mut self) -> DiagnosticsDecorator<'_> {
        DiagnosticsDecorator { d: self }
    }

    /// Begins a node named `name`, recording the layout protocol currently in effect.
    pub fn node(&mut self, name: impl Into<Cow<'static, str>>) -> DiagnosticsNodeBuilder<'_> {
        let protocol = self.protocol;

        let decorations = std::mem::take(&mut self.pending_decorations);

        DiagnosticsNodeBuilder {
            d: self,

            name: name.into(),
            protocol,

            properties: Vec::new(),
            decorations,

            children: Vec::new(),
        }
    }

    /// Begins a node named after the type `T`, recording the layout protocol currently in effect.
    pub fn node_for<T: ?Sized>(&mut self) -> DiagnosticsNodeBuilder<'_> {
        let protocol = self.protocol;

        let decorations = std::mem::take(&mut self.pending_decorations);

        DiagnosticsNodeBuilder {
            d: self,

            name: Cow::Borrowed(Diagnostics::short_type_name::<T>()),
            protocol,

            properties: Vec::new(),
            decorations,

            children: Vec::new(),
        }
    }

    /// The bare name of `T`, without its module path or generic arguments.
    pub fn short_type_name<T: ?Sized>() -> &'static str {
        trim_type_name(std::any::type_name::<T>())
    }
}

/// Drops the module path and generic arguments from a [`std::any::type_name`] string.
fn trim_type_name(full: &str) -> &str {
    let base = full.split('<').next().unwrap_or(full);
    base.rsplit("::").next().unwrap_or(base)
}

/// A node being assembled within a [`Diagnostics`] capture.
pub struct DiagnosticsNodeBuilder<'a> {
    d: &'a mut Diagnostics,

    name: Cow<'static, str>,
    protocol: Option<ProtocolTag>,

    properties: Vec<DiagnosticsProperty>,
    decorations: Vec<DiagnosticsProperty>,

    children: Vec<DiagnosticsNode>,
}

impl DiagnosticsNodeBuilder<'_> {
    /// Adds the property `name` with `value` rendered to text.
    pub fn property(mut self, name: &'static str, value: impl fmt::Debug) -> Self {
        self.properties.push(DiagnosticsProperty {
            name,
            value: format!("{value:?}"),
        });

        self
    }

    /// Adds the property `name` rendered to text, only when `value` is `Some`.
    pub fn property_opt(self, name: &'static str, value: Option<impl fmt::Debug>) -> Self {
        match value {
            Some(value) => self.property(name, value),
            None => self,
        }
    }

    /// Adds the flag `name`, rendered as a bare name, only when `set`.
    pub fn flag(mut self, name: &'static str, set: bool) -> Self {
        if set {
            self.properties.push(DiagnosticsProperty {
                name,
                value: String::new(),
            });
        }

        self
    }

    /// Builds a child that uses the same layout protocol as this node, attaching it as the last child.
    pub fn child(mut self, f: impl FnOnce(&mut Diagnostics) -> DiagnosticsNode) -> Self {
        self.children.push(f(self.d));
        self
    }

    /// Builds a child labeled with the `field` it occupies in this node, attaching it as the last
    /// child.
    pub fn child_named(
        mut self,
        field: &'static str,
        f: impl FnOnce(&mut Diagnostics) -> DiagnosticsNode,
    ) -> Self {
        let mut child = f(self.d);
        child.field = Some(field);
        self.children.push(child);
        self
    }

    /// Builds a child subtree that uses a different layout `protocol`, attaching it as the last child.
    pub fn child_in(
        mut self,
        protocol: Option<ProtocolTag>,
        f: impl FnOnce(&mut Diagnostics) -> DiagnosticsNode,
    ) -> Self {
        let old = std::mem::replace(&mut self.d.protocol, protocol);
        {
            self.children.push(f(self.d));
        }
        self.d.protocol = old;

        self
    }

    /// Completes the node, yielding the snapshot.
    pub fn finish(self) -> DiagnosticsNode {
        DiagnosticsNode {
            name: self.name,
            field: None,
            protocol: self.protocol,

            properties: self.properties,
            decorations: self.decorations,

            children: self.children,
        }
    }
}

/// A chain that annotates the child built through it with decorations its node carries.
///
/// Decorations are a parent's notes on a child, so a node never decorates itself: the properties
/// added here belong to the node built by [`child`](Self::child), not to the one that opened the
/// chain.
pub struct DiagnosticsDecorator<'a> {
    d: &'a mut Diagnostics,
}

impl DiagnosticsDecorator<'_> {
    /// Adds a decoration `name` with `value` rendered to text.
    pub fn property(self, name: &'static str, value: impl fmt::Debug) -> Self {
        self.d.pending_decorations.push(DiagnosticsProperty {
            name,
            value: format!("{value:?}"),
        });

        self
    }

    /// Adds a decoration `name` rendered to text, only when `value` is `Some`.
    pub fn property_opt(self, name: &'static str, value: Option<impl fmt::Debug>) -> Self {
        match value {
            Some(value) => self.property(name, value),
            None => self,
        }
    }

    /// Adds the flag decoration `name`, rendered as a bare name, only when `set`.
    pub fn flag(self, name: &'static str, set: bool) -> Self {
        if set {
            self.d.pending_decorations.push(DiagnosticsProperty {
                name,
                value: String::new(),
            });
        }

        self
    }

    /// Builds the child, whose node claims the decorations.
    pub fn child(self, f: impl FnOnce(&mut Diagnostics) -> DiagnosticsNode) -> DiagnosticsNode {
        f(self.d)
    }
}

/// A snapshot of one node in a tree, captured for inspection.
///
/// A node carries a display name, the layout protocol it lays out under, the named property values
/// that describe it, and the snapshots of its children. It is an owned value with no ties to the tree
/// it was captured from and stays valid after that tree changes.
///
/// Its [`Display`] form is the full tree, one node per line, with box-drawing characters connecting
/// parents to children. A node whose protocol differs from its parent's is preceded by a divider line
/// naming the transition.
pub struct DiagnosticsNode {
    name: Cow<'static, str>,
    field: Option<&'static str>,
    protocol: Option<ProtocolTag>,

    properties: Vec<DiagnosticsProperty>,
    decorations: Vec<DiagnosticsProperty>,

    children: Vec<DiagnosticsNode>,
}

impl DiagnosticsNode {
    /// The node's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The field this node occupies in its parent, if the parent named it.
    pub fn field(&self) -> Option<&'static str> {
        self.field
    }

    /// The layout protocol this node lays out under, if any.
    pub fn protocol(&self) -> Option<ProtocolTag> {
        self.protocol
    }

    /// The node's properties, in the order they were added.
    pub fn properties(&self) -> &[DiagnosticsProperty] {
        &self.properties
    }

    /// The node's children, in order.
    pub fn children(&self) -> &[DiagnosticsNode] {
        &self.children
    }

    fn write_node(&self, f: &mut fmt::Formatter<'_>, prefix: &str, pretty: bool) -> fmt::Result {
        if let Some(field) = self.field {
            write!(f, "{field}: ")?;
        }

        f.write_str(&self.name)?;

        if pretty {
            let cont = if self.children.is_empty() {
                "   "
            } else {
                "│  "
            };

            for property in &self.properties {
                write!(f, "\n{prefix}{cont}")?;
                write_entry(f, "", property)?;
            }

            if let Some((first, rest)) = self.decorations.split_first() {
                write!(f, "\n{prefix}{cont}{{")?;
                write_entry(f, "", first)?;
                for decoration in rest {
                    write_entry(f, ", ", decoration)?;
                }
                f.write_char('}')?;
            }

            let has_fields = !self.properties.is_empty() || !self.decorations.is_empty();
            if has_fields && !self.children.is_empty() {
                write!(f, "\n{prefix}│")?;
            }
        } else {
            for property in &self.properties {
                write_entry(f, "  ", property)?;
            }

            if let Some((first, rest)) = self.decorations.split_first() {
                f.write_str("  {")?;
                write_entry(f, "", first)?;
                for decoration in rest {
                    write_entry(f, ", ", decoration)?;
                }
                f.write_char('}')?;
            }
        }

        Ok(())
    }

    fn write_tree(&self, f: &mut fmt::Formatter<'_>, prefix: &str, pretty: bool) -> fmt::Result {
        self.write_node(f, prefix, pretty)?;

        for (index, child) in self.children.iter().enumerate() {
            let last = index + 1 == self.children.len();

            if let Some(protocol) = child.protocol.filter(|&p| Some(p) != self.protocol) {
                f.write_char('\n')?;
                f.write_str(prefix)?;
                f.write_str("╞")?;

                match self.protocol {
                    Some(from) => {
                        write!(f, "═══════ {} → {} ═══════", from.name(), protocol.name())?;
                    }
                    None => write!(f, "═══════ {} ═══════", protocol.name())?,
                }
            }

            f.write_char('\n')?;
            f.write_str(prefix)?;
            f.write_str(if last { "└─ " } else { "├─ " })?;

            let child_prefix = if last { "   " } else { "│  " };
            child.write_tree(f, &format!("{prefix}{child_prefix}"), pretty)?;
        }

        Ok(())
    }
}

/// Writes `property` after `separator`, as `name=value`, or as a bare `name` when its value is empty.
fn write_entry(
    f: &mut fmt::Formatter<'_>,
    separator: &str,
    property: &DiagnosticsProperty,
) -> fmt::Result {
    f.write_str(separator)?;
    f.write_str(property.name)?;

    if !property.value.is_empty() {
        write!(f, "={}", property.value)?;
    }

    Ok(())
}

impl Display for DiagnosticsNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_tree(f, "", f.alternate())
    }
}

/// A named value describing one aspect of a [`DiagnosticsNode`], already rendered to text.
pub struct DiagnosticsProperty {
    name: &'static str,
    value: String,
}

impl DiagnosticsProperty {
    /// The property's name.
    pub fn name(&self) -> &str {
        self.name
    }

    /// The property's value as text.
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostics, ProtocolTag};

    #[test]
    fn leaf_renders_name_only() {
        let mut d = Diagnostics::new();

        assert_eq!(d.node("Leaf").finish().to_string(), "Leaf");
    }

    #[test]
    fn properties_render_inline_in_order() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Box")
            .property("size", "100.0x40.0")
            .property("painted", true)
            .finish();

        assert_eq!(node.to_string(), "Box  size=\"100.0x40.0\"  painted=true");
    }

    #[test]
    fn property_opt_skips_none() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Box")
            .property_opt("width", Some(16))
            .property_opt("height", None::<u32>)
            .finish();

        assert_eq!(node.to_string(), "Box  width=16");
    }

    #[test]
    fn flag_renders_bare_and_skips_when_unset() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Box")
            .flag("painted", true)
            .flag("clipped", false)
            .finish();

        assert_eq!(node.to_string(), "Box  painted");
    }

    #[test]
    fn decorations_render_in_a_trailing_group() {
        let mut d = Diagnostics::new();

        let node = d
            .decorate()
            .flag("parent_uses_size", true)
            .property("relayout_boundary", "registered")
            .child(|d| d.node("RenderSizedBox").property("width", 16).finish());

        assert_eq!(
            node.to_string(),
            "RenderSizedBox  width=16  {parent_uses_size, relayout_boundary=\"registered\"}"
        );
    }

    #[test]
    fn children_connect_with_branches() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Parent")
            .child(|d| d.node("First").finish())
            .child(|d| d.node("Last").finish())
            .finish();

        assert_eq!(
            node.to_string(),
            "Parent\n\
             ├─ First\n\
             └─ Last",
        );
    }

    #[test]
    fn sibling_after_nested_child_keeps_continuation_line() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Root")
            .child(|d| {
                d.node("Branch")
                    .child(|d| d.node("DeepLeaf").finish())
                    .finish()
            })
            .child(|d| d.node("Sibling").finish())
            .finish();

        assert_eq!(
            node.to_string(),
            "Root\n\
             ├─ Branch\n\
             │  └─ DeepLeaf\n\
             └─ Sibling",
        );
    }

    #[test]
    fn last_child_indents_without_continuation_line() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Root")
            .child(|d| {
                d.node("Branch")
                    .child(|d| d.node("DeepLeaf").finish())
                    .finish()
            })
            .finish();

        assert_eq!(
            node.to_string(),
            "Root\n\
             └─ Branch\n   \
                └─ DeepLeaf",
        );
    }

    #[test]
    fn a_named_child_shows_its_field() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Button")
            .child_named("icon", |d| d.node("RenderIcon").finish())
            .child(|d| d.node("RenderLabel").finish())
            .finish();

        assert_eq!(
            node.to_string(),
            "Button\n\
             ├─ icon: RenderIcon\n\
             └─ RenderLabel",
        );
        assert_eq!(node.children()[0].field(), Some("icon"));
        assert_eq!(node.children()[1].field(), None);
    }

    #[test]
    fn alternate_form_puts_each_field_on_its_own_line() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Counter")
            .property("count", 7)
            .property("enabled", true)
            .child(|d| d.node("RenderBox").property("size", "10x20").finish())
            .finish();

        assert_eq!(
            format!("{node:#}"),
            "Counter\n\
             │  count=7\n\
             │  enabled=true\n\
             │\n\
             └─ RenderBox\n      \
                size=\"10x20\"",
        );
        assert_eq!(
            node.to_string(),
            "Counter  count=7  enabled=true\n\
             └─ RenderBox  size=\"10x20\"",
        );
    }

    #[test]
    fn short_type_name_strips_path_and_generics() {
        assert_eq!(
            super::trim_type_name("alloc::vec::Vec<core::option::Option<u32>>"),
            "Vec"
        );
        assert_eq!(super::trim_type_name("core::option::Option"), "Option");
        assert_eq!(super::trim_type_name("Plain"), "Plain");

        assert_eq!(Diagnostics::short_type_name::<Vec<Option<u32>>>(), "Vec");
    }

    #[test]
    fn node_for_uses_short_name() {
        let mut d = Diagnostics::new();

        assert_eq!(d.node_for::<Vec<Option<u32>>>().finish().name(), "Vec");
    }

    #[test]
    fn builder_chain_assembles_properties_and_children() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Column")
            .property("children", 2)
            .child(|d| d.node("Padding").property("insets", 8).finish())
            .child(|d| d.node("SizedBox").finish())
            .finish();

        assert_eq!(
            node.to_string(),
            "Column  children=2\n\
             ├─ Padding  insets=8\n\
             └─ SizedBox",
        );
    }

    #[test]
    fn child_in_sets_the_protocol_for_its_subtree() {
        let mut d = Diagnostics::new();

        let node = d
            .node("RenderViewport")
            .property("axis", "vertical")
            .child_in(Some(ProtocolTag::SLIVER), |d| {
                d.node("RenderSliverList")
                    .child(|d| d.node("RenderSliverFixed").finish())
                    .finish()
            })
            .finish();

        assert_eq!(node.protocol(), None);
        assert_eq!(node.children()[0].protocol(), Some(ProtocolTag::SLIVER));
        assert_eq!(
            node.children()[0].children()[0].protocol(),
            Some(ProtocolTag::SLIVER)
        );
    }

    #[test]
    fn protocol_swap_renders_a_divider_above_the_node() {
        let mut d = Diagnostics::new();

        let node = d
            .node("RenderViewport")
            .child_in(Some(ProtocolTag::SLIVER), |d| {
                d.node("RenderSliverList")
                    .property("axis", "vertical")
                    .child(|d| d.node("RenderSliverFixed").finish())
                    .finish()
            })
            .finish();

        assert_eq!(
            node.to_string(),
            "RenderViewport\n\
             ╞═══════ sliver ═══════\n\
             └─ RenderSliverList  axis=\"vertical\"\n   \
                └─ RenderSliverFixed",
        );
    }

    #[test]
    fn protocol_swap_divider_names_the_transition_when_the_parent_has_one() {
        let mut d = Diagnostics::new();

        let node = d
            .node("Root")
            .child_in(Some(ProtocolTag::BOX), |d| {
                d.node("BoxNode")
                    .child_in(Some(ProtocolTag::SLIVER), |d| d.node("SliverNode").finish())
                    .finish()
            })
            .finish();

        assert_eq!(
            node.to_string(),
            "Root\n\
             ╞═══════ box ═══════\n\
             └─ BoxNode\n   \
                ╞═══════ box → sliver ═══════\n   \
                └─ SliverNode",
        );
    }
}
