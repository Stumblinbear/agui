use accesskit::Role;
use peniko::kurbo::Affine;

use crate::{
    geometry::{Offset, Projection, Rect, Size},
    semantics::{
        config::{SemanticsConfig, SemanticsNodeId},
        merge::{control_value, merge_node, names_from_content},
        tree::SemanticsNode,
    },
};

/// Assembles the nodes of a [`SemanticsTree`](crate::semantics::SemanticsTree) during a walk. A render object
/// adds its contribution with [`node`](Self::node). The builder folds it into the nearest enclosing node that
/// accepts it, or starts a new node when none does.
pub struct SemanticsTreeBuilder<'a> {
    counter: &'a mut u64,
    stack: Vec<Frame>,
    roots: Vec<SemanticsNode>,
    /// The transform accumulated since the enclosing node, snapshotted onto each node as it opens and reset
    /// for that node's subtree.
    projection: Projection,
    /// The clip in the current coordinate space, or `None` for unbounded. A node outside it is hidden.
    clip: Option<Rect>,
}

/// A node under construction: the host that descendants absorb into, plus the state for computing its name.
#[allow(clippy::struct_excessive_bools)]
struct Frame {
    id: SemanticsNodeId,
    config: SemanticsConfig,
    role: Role,
    has_explicit_label: bool,
    /// The text gathered from absorbed descendants, joined into the name when the node has no explicit label
    /// and its role is named from content.
    content: Vec<String>,
    /// Set while absorbing the subtree of a descendant that carries its own name, so that subtree adds
    /// nothing further to the host's name.
    suppress_name: bool,
    accepts_merge: bool,
    collapsing: bool,
    children: Vec<SemanticsNode>,
    /// The transform from this node's space to its parent node's, snapshotted when it opened.
    projection: Projection,
    size: Size,
    hidden: bool,
}

impl Frame {
    fn finish(mut self) -> SemanticsNode {
        if !self.has_explicit_label && names_from_content(self.role) && !self.content.is_empty() {
            self.config.node.set_label(self.content.join(" "));
        }

        // A pure offset folds into the bounds (parent space, no transform). A real matrix is set as the
        // transform, leaving the bounds in own space.
        match self.projection {
            Projection::Offset(offset) => {
                self.config
                    .node
                    .set_bounds((Rect::from(self.size) + offset).into());
            }
            Projection::Affine(affine) => {
                self.config
                    .node
                    .set_transform(accesskit::Affine::new(affine.as_coeffs()));
                self.config.node.set_bounds(Rect::from(self.size).into());
            }
        }

        if self.hidden {
            self.config.node.set_hidden();
        }

        SemanticsNode {
            id: self.id,
            config: self.config,
            children: self.children,
        }
    }
}

impl<'a> SemanticsTreeBuilder<'a> {
    /// A builder that mints ids by advancing `counter`, the pipeline's monotonic id source. Each node it opens
    /// takes the next value, so a node minted on a later walk cannot collide with one minted earlier.
    pub fn new(counter: &'a mut u64) -> Self {
        Self {
            counter,
            stack: Vec::new(),
            roots: Vec::new(),
            projection: Projection::default(),
            clip: None,
        }
    }

    /// The next id, advancing the counter.
    fn mint(&mut self) -> SemanticsNodeId {
        let next = *self.counter;
        *self.counter += 1;
        SemanticsNodeId(next)
    }

    /// Adds the contribution `config`, with `build` recording its subtree. The contribution folds into the
    /// nearest enclosing node when that node accepts merges and this is not its own boundary. Otherwise it
    /// starts a new node whose id is `id`, minted into the slot the first time and reused on later walks so
    /// the node keeps its identity across frames. An excluded config, and its subtree, contribute nothing.
    pub fn node(
        &mut self,
        id: &mut Option<SemanticsNodeId>,
        config: SemanticsConfig,
        size: Size,
        build: impl FnOnce(&mut SemanticsTreeBuilder<'_>),
    ) {
        if config.excluded {
            return;
        }

        let absorb = self
            .stack
            .last()
            .is_some_and(|top| top.collapsing || (top.accepts_merge && !config.is_boundary));

        if absorb {
            self.absorb(&config, build);
        } else {
            let id = *id.get_or_insert_with(|| self.mint());
            self.open(id, config, size, build);
        }
    }

    /// Runs `build` with `offset` applied to the geometry of the nodes it records.
    pub fn with_offset(
        &mut self,
        offset: Offset,
        build: impl FnOnce(&mut SemanticsTreeBuilder<'_>),
    ) {
        let projection = self.projection;
        let clip = self.clip;
        self.projection = self.projection.then_offset(offset);
        self.clip = self.clip.map(|clip| clip + (-offset));
        build(self);
        self.projection = projection;
        self.clip = clip;
    }

    /// Runs `build` with `transform` applied to the geometry of the nodes it records.
    pub fn with_transform(
        &mut self,
        transform: Affine,
        build: impl FnOnce(&mut SemanticsTreeBuilder<'_>),
    ) {
        let projection = self.projection;
        let clip = self.clip;
        self.projection = self.projection.then_affine(transform);
        self.clip = self
            .clip
            .map(|clip| clip.transform_bbox(transform.inverse()));
        build(self);
        self.projection = projection;
        self.clip = clip;
    }

    /// Runs `build` clipped to `clip`, in the current space. A node fully outside the clip is hidden.
    pub fn with_clip(&mut self, clip: Rect, build: impl FnOnce(&mut SemanticsTreeBuilder<'_>)) {
        let previous = self.clip;

        self.clip = Some(match previous {
            Some(current) => current.intersect(clip),
            None => clip,
        });

        build(self);

        self.clip = previous;
    }

    /// The nodes recorded at the top level.
    #[must_use]
    pub fn finish(self) -> Vec<SemanticsNode> {
        debug_assert!(self.stack.is_empty(), "a node was left open");
        self.roots
    }

    fn is_clipped_out(&self, size: Size) -> bool {
        self.clip
            .is_some_and(|clip| !clip.intersects(Rect::from(size)))
    }

    fn open(
        &mut self,
        id: SemanticsNodeId,
        config: SemanticsConfig,
        size: Size,
        build: impl FnOnce(&mut SemanticsTreeBuilder<'_>),
    ) {
        let role = config.node.role();
        let has_explicit_label = config
            .node
            .label()
            .is_some_and(|label| !label.trim().is_empty());
        let accepts_merge = !config.explicit_children;
        let collapsing = config.merge_descendants;
        let hidden = self.is_clipped_out(size);

        self.stack.push(Frame {
            id,
            config,
            role,
            has_explicit_label,
            content: Vec::new(),
            suppress_name: false,
            accepts_merge,
            collapsing,
            children: Vec::new(),
            projection: self.projection,
            size,
            hidden,
        });

        // The subtree's transforms are relative to this node, not the one above it.
        let parent = std::mem::replace(&mut self.projection, Projection::IDENTITY);
        build(self);
        self.projection = parent;

        let node = self.stack.pop().expect("the frame just pushed").finish();
        match self.stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => self.roots.push(node),
        }
    }

    fn absorb(
        &mut self,
        config: &SemanticsConfig,
        build: impl FnOnce(&mut SemanticsTreeBuilder<'_>),
    ) {
        let top = self
            .stack
            .last_mut()
            .expect("absorb runs under an open node");

        merge_node(&mut top.config.node, &config.node);

        let previous = top.suppress_name;

        let gathering =
            !top.has_explicit_label && names_from_content(top.role) && !top.suppress_name;

        let child_suppresses = if gathering {
            if let Some(label) = config.node.label().map(str::trim).filter(|l| !l.is_empty()) {
                top.content.push(label.to_owned());
                true
            } else if let Some(value) = control_value(&config.node) {
                top.content.push(value);
                true
            } else {
                false
            }
        } else {
            true
        };

        top.suppress_name = previous || child_suppresses;

        build(self);

        self.stack
            .last_mut()
            .expect("the open node outlives its subtree")
            .suppress_name = previous;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(text: &str) -> SemanticsConfig {
        let mut config = SemanticsConfig::new(Role::Label);
        config.node.set_label(text);
        config
    }

    /// Adds a node, minting its id, the way a render object would.
    fn node(
        builder: &mut SemanticsTreeBuilder<'_>,
        config: SemanticsConfig,
        build: impl FnOnce(&mut SemanticsTreeBuilder<'_>),
    ) {
        builder.node(&mut None, config, Size::ZERO, build);
    }

    fn build(f: impl FnOnce(&mut SemanticsTreeBuilder<'_>)) -> Vec<SemanticsNode> {
        let mut counter = 0;
        let mut builder = SemanticsTreeBuilder::new(&mut counter);
        f(&mut builder);
        builder.finish()
    }

    #[test]
    fn siblings_at_the_top_level_each_get_a_node() {
        let nodes = build(|b| {
            node(b, label("first"), |_| {});
            node(b, label("second"), |_| {});
        });

        assert_eq!(nodes.len(), 2);
        assert_eq!(nodes[0].config.node.label(), Some("first"));
        assert_eq!(nodes[1].config.node.label(), Some("second"));
    }

    #[test]
    fn each_node_gets_a_distinct_id() {
        let nodes = build(|b| {
            node(b, label("first"), |_| {});
            node(b, label("second"), |_| {});
        });

        assert_ne!(nodes[0].id, nodes[1].id);
    }

    #[test]
    fn a_name_from_content_host_absorbs_its_child() {
        let nodes = build(|b| {
            node(b, SemanticsConfig::new(Role::Button), |b| {
                node(b, label("Submit"), |_| {});
            });
        });

        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].children.len(), 0, "the label was absorbed");
        assert_eq!(nodes[0].accessible_name().as_deref(), Some("Submit"));
    }

    #[test]
    fn explicit_children_keeps_children_as_their_own_nodes() {
        let mut list = SemanticsConfig::new(Role::List);
        list.explicit_children = true;

        let nodes = build(|b| {
            node(b, list, |b| {
                node(b, label("item"), |_| {});
            });
        });

        assert_eq!(nodes[0].config.node.role(), Role::List);
        assert_eq!(nodes[0].children.len(), 1);
        assert_eq!(nodes[0].children[0].config.node.label(), Some("item"));
    }

    #[test]
    fn excluded_config_drops_its_subtree() {
        let nodes = build(|b| {
            node(
                b,
                SemanticsConfig {
                    excluded: true,
                    ..Default::default()
                },
                |b| node(b, label("hidden"), |_| {}),
            );
        });

        assert!(nodes.is_empty());
    }

    #[test]
    fn name_from_content_joins_absorbed_siblings_in_order() {
        let nodes = build(|b| {
            node(b, SemanticsConfig::new(Role::Button), |b| {
                node(b, label("icon"), |_| {});
                node(b, label("Submit"), |_| {});
            });
        });

        assert_eq!(nodes[0].accessible_name().as_deref(), Some("icon Submit"));
    }

    #[test]
    fn an_explicit_label_overrides_content() {
        let mut button = SemanticsConfig::new(Role::Button);
        button.node.set_label("Save");

        let nodes = build(|b| node(b, button, |b| node(b, label("disk icon"), |_| {})));

        assert_eq!(nodes[0].accessible_name().as_deref(), Some("Save"));
    }

    #[test]
    fn an_explicit_label_stops_the_descent_into_its_subtree() {
        let nodes = build(|b| {
            node(b, SemanticsConfig::new(Role::Button), |b| {
                node(b, label("Save"), |b| node(b, label("disk icon"), |_| {}));
            });
        });

        assert_eq!(nodes[0].accessible_name().as_deref(), Some("Save"));
    }

    #[test]
    fn a_role_not_named_from_content_takes_no_name_from_its_subtree() {
        let nodes = build(|b| {
            node(b, SemanticsConfig::new(Role::Group), |b| {
                node(b, label("item"), |_| {});
            });
        });

        assert_eq!(nodes[0].accessible_name(), None);
    }

    #[test]
    fn an_embedded_control_contributes_its_value() {
        let mut field = SemanticsConfig::new(Role::TextInput);
        field.node.set_value("Ada");

        let nodes = build(|b| {
            node(b, SemanticsConfig::new(Role::Button), |b| {
                node(b, label("Name:"), |_| {});
                node(b, field, |_| {});
            });
        });

        assert_eq!(nodes[0].accessible_name().as_deref(), Some("Name: Ada"));
    }

    #[test]
    fn an_absorbed_child_contributes_its_value() {
        let mut slider = SemanticsConfig::new(Role::Slider);
        slider.node.set_label("Volume");

        let mut reading = SemanticsConfig::new(Role::Label);
        reading.node.set_value("50%");

        let nodes = build(|b| {
            node(b, slider, |b| node(b, reading, |_| {}));
        });

        assert_eq!(nodes[0].accessible_name().as_deref(), Some("Volume"));
        assert_eq!(nodes[0].config.node.value(), Some("50%"));
    }

    #[test]
    fn a_node_takes_its_bounds_from_its_size() {
        let nodes = build(|b| {
            b.node(&mut None, label("x"), Size::new(30.0_f32, 10.0), |_| {});
        });

        assert_eq!(
            nodes[0].config.node.bounds(),
            Some(accesskit::Rect::new(0.0, 0.0, 30.0, 10.0)),
        );
        assert!(nodes[0].config.node.transform().is_none());
    }

    #[test]
    fn an_offset_folds_into_the_bounds_with_no_transform() {
        let nodes = build(|b| {
            b.with_offset(Offset::from((5.0_f32, 7.0)), |b| {
                b.node(&mut None, label("x"), Size::new(30.0_f32, 10.0), |_| {});
            });
        });

        assert_eq!(
            nodes[0].config.node.bounds(),
            Some(accesskit::Rect::new(5.0, 7.0, 35.0, 17.0)),
        );
        assert!(nodes[0].config.node.transform().is_none());
    }

    #[test]
    fn nested_offsets_accumulate() {
        let nodes = build(|b| {
            b.with_offset(Offset::from((5.0_f32, 0.0)), |b| {
                b.with_offset(Offset::from((3.0_f32, 2.0)), |b| {
                    b.node(&mut None, label("x"), Size::new(10.0_f32, 10.0), |_| {});
                });
            });
        });

        assert_eq!(
            nodes[0].config.node.bounds(),
            Some(accesskit::Rect::new(8.0, 2.0, 18.0, 12.0)),
        );
    }

    #[test]
    fn a_real_transform_is_set_with_own_space_bounds() {
        let nodes = build(|b| {
            b.with_transform(Affine::scale(2.0), |b| {
                b.node(&mut None, label("x"), Size::new(10.0_f32, 10.0), |_| {});
            });
        });

        assert!(nodes[0].config.node.transform().is_some());
        assert_eq!(
            nodes[0].config.node.bounds(),
            Some(accesskit::Rect::new(0.0, 0.0, 10.0, 10.0)),
        );
    }

    #[test]
    fn a_node_outside_the_clip_is_hidden() {
        let nodes = build(|b| {
            b.with_clip(Rect::new(100.0_f32, 0.0, 10.0_f32, 10.0), |b| {
                b.node(&mut None, label("x"), Size::new(10.0_f32, 10.0), |_| {});
            });
        });

        assert!(nodes[0].config.node.is_hidden());
    }

    #[test]
    fn a_node_inside_the_clip_is_not_hidden() {
        let nodes = build(|b| {
            b.with_clip(Rect::new(0.0_f32, 0.0, 50.0_f32, 50.0), |b| {
                b.node(&mut None, label("x"), Size::new(10.0_f32, 10.0), |_| {});
            });
        });

        assert!(!nodes[0].config.node.is_hidden());
    }

    #[test]
    fn a_nested_node_is_positioned_relative_to_its_parent_node() {
        let nodes = build(|b| {
            b.with_offset(Offset::from((10.0_f32, 0.0)), |b| {
                let mut outer = SemanticsConfig::new(Role::Group);
                outer.explicit_children = true;
                b.node(&mut None, outer, Size::new(100.0_f32, 100.0), |b| {
                    b.with_offset(Offset::from((5.0_f32, 5.0)), |b| {
                        b.node(&mut None, label("inner"), Size::new(10.0_f32, 10.0), |_| {});
                    });
                });
            });
        });

        assert_eq!(
            nodes[0].config.node.bounds(),
            Some(accesskit::Rect::new(10.0, 0.0, 110.0, 100.0)),
        );
        // The inner node's bounds are relative to the outer node, not the root: (5, 5), not (15, 5).
        assert_eq!(
            nodes[0].children[0].config.node.bounds(),
            Some(accesskit::Rect::new(5.0, 5.0, 15.0, 15.0)),
        );
    }

    #[test]
    fn an_offset_under_a_transform_is_folded_into_the_transform() {
        let nodes = build(|b| {
            b.with_transform(Affine::scale(2.0), |b| {
                b.with_offset(Offset::from((5.0_f32, 0.0)), |b| {
                    b.node(&mut None, label("x"), Size::new(10.0_f32, 10.0), |_| {});
                });
            });
        });

        assert!(nodes[0].config.node.transform().is_some());
    }

    #[test]
    fn a_node_added_on_a_later_walk_does_not_reuse_an_existing_id() {
        // One shared counter, as the pipeline holds, persists across both walks.
        let mut counter = 0;

        // The render objects keep their id slots across walks, as real render objects do.
        let mut existing_id = None;
        let mut added_id = None;

        // Walk 1: only the first render object emits.
        {
            let mut b = SemanticsTreeBuilder::new(&mut counter);
            b.node(&mut existing_id, label("existing"), Size::ZERO, |_| {});
            let _ = b.finish();
        }

        // Walk 2: the first render object reuses its id, and a second one emits for the first time.
        let nodes = {
            let mut b = SemanticsTreeBuilder::new(&mut counter);
            b.node(&mut existing_id, label("existing"), Size::ZERO, |_| {});
            b.node(&mut added_id, label("added"), Size::ZERO, |_| {});
            b.finish()
        };

        assert_eq!(existing_id, Some(SemanticsNodeId(0)));
        assert_eq!(added_id, Some(SemanticsNodeId(1)));
        assert_ne!(nodes[0].id, nodes[1].id);
    }
}
