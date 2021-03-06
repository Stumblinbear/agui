use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{
    engine::Modify,
    widget::{Widget, WidgetId},
};

use super::Engine;

const RESET: &str = "\u{001b}[0m";
const GRAY: &str = "\u{001b}[30;1m";
const RED: &str = "\u{001b}[31;1m";
const GREEN: &str = "\u{001b}[32;1m";
const YELLOW: &str = "\u{001b}[33;1m";
// const BLUE: &str = "\u{001b}[34;1m";
// const MAGENTA: &str = "\u{001b}[35;1m";
const CYAN: &str = "\u{001b}[36;1m";
const WHITE: &str = "\u{001b}[37;1m";

pub fn print_tree(engine: &Engine<'_>) {
    println!("Tree:");

    let tree = engine.get_tree();

    for widget_id in tree.iter_down(None) {
        let node = tree.get_node(widget_id).expect("broken tree");

        let depth = node.depth;

        print_node(
            depth,
            Some(widget_id),
            &node.widget.get(),
            WHITE,
            &format!("{} {:?} {:?}", GRAY, node.layout_type, node.layout),
        );
    }
}

pub fn print_tree_modifications(engine: &Engine<'_>) {
    println!("Tree:");

    let tree = engine.get_tree();

    let mods = &engine.modifications;

    let mut new_root = None;
    let mut spawns = HashMap::new();
    let mut rebuilds = HashSet::new();
    let mut destroys = HashSet::new();

    for modify in mods {
        match modify {
            Modify::Spawn(parent_id, widget) => match parent_id {
                Some(parent_id) => {
                    if !spawns.contains_key(parent_id) {
                        spawns.insert(*parent_id, Vec::new());
                    }

                    spawns.get_mut(parent_id).unwrap().push(widget.get());
                }
                None => {
                    new_root = Some(widget);
                }
            },
            Modify::Rebuild(widget_id) => {
                rebuilds.insert(*widget_id);
            }
            Modify::Destroy(widget_id) => {
                destroys.insert(*widget_id);
            }
        }
    }

    // No widgets are added to the tree
    if tree.get_root().is_none() {
        // If we have a new root widget queued, print it
        if let Some(widget) = new_root {
            print_node(0, None, &widget.get(), GREEN, "");
        }

        return;
    }

    for widget_id in tree.iter_down(None) {
        let node = tree.get_node(widget_id).expect("broken tree");

        let depth = node.depth;

        let is_rebuild_queued = rebuilds.contains(&widget_id);

        let is_destroy_queued = if depth == 0 && new_root.is_some() {
            true
        } else {
            destroys.contains(&widget_id)
        };

        print_node(
            depth,
            Some(widget_id),
            &node.widget.get(),
            if is_destroy_queued {
                RED
            } else if is_rebuild_queued {
                YELLOW
            } else {
                WHITE
            },
            &format!("{} {:?} {:?}", GRAY, node.layout_type, node.layout),
        );

        for to_spawn in spawns.get(&widget_id).unwrap_or(&Vec::new()) {
            print_node(depth + 1, None, to_spawn, GREEN, "");
        }
    }
}

fn print_node(
    depth: usize,
    widget_id: Option<WidgetId>,
    widget: &Rc<dyn Widget>,
    color: &'static str,
    suffix: &str,
) {
    print!("{}", GRAY);

    if depth > 0 {
        print!("{}", "|     ".repeat(depth / 3));

        if depth % 3 == 1 {
            print!("| ");
        } else if depth % 3 == 2 {
            print!("|   ");
        }
    }

    print!("{}", RESET);

    print!("{}", color);

    print!("{}", widget.get_type_name());

    print!("{}", GRAY);

    if let Some(widget_id) = widget_id {
        print!(" (#{:?})", widget_id);
    }

    println!(" {}", suffix);

    print!("{}", RESET);
}
