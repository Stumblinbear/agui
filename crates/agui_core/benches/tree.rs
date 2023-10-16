use criterion::{black_box, criterion_group, criterion_main, Criterion};

use agui_core::{element::ElementId, util::tree::Tree};

fn tree_ops(c: &mut Criterion) {
    c.bench_function("add to tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<ElementId, usize>::default();

                let root_id = tree.add(None, 0);

                (tree, root_id)
            },
            |(mut tree, root_id)| {
                for i in 0..1000 {
                    tree.add(Some(root_id), i);
                }
            },
        )
    });

    c.bench_function("remove from tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<ElementId, usize>::default();

                let mut widget_ids = Vec::new();

                let root_id = tree.add(None, 0);

                widget_ids.push(root_id);

                for i in 0..1000 {
                    widget_ids.push(tree.add(Some(root_id), i));
                }

                (tree, widget_ids)
            },
            |(mut tree, widget_ids)| {
                for widget_id in widget_ids {
                    tree.remove(widget_id, false);
                }
            },
        )
    });
}

fn tree_iter_down(c: &mut Criterion) {
    c.bench_function("iterate down tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<ElementId, usize>::default();

                let parent_id = tree.add(None, 0);
                let parent_id = tree.add(Some(parent_id), 1);
                let parent_id = tree.add(Some(parent_id), 2);
                tree.add(Some(parent_id), 3);

                tree
            },
            |tree| {
                for _ in 0..1000 {
                    let mut walker = tree.iter_down();

                    while black_box(walker.next().is_some()) {}
                }
            },
        )
    });
}

fn tree_iter_up(c: &mut Criterion) {
    c.bench_function("iterate up tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<ElementId, usize>::default();

                let parent_id = tree.add(None, 0);
                let widget_id = tree.add(Some(parent_id), 1);
                let parent_id = tree.add(Some(widget_id), 2);
                tree.add(Some(parent_id), 3);

                tree
            },
            |tree| {
                for _ in 0..1000 {
                    let mut walker = tree.iter_up();

                    while black_box(walker.next().is_some()) {}
                }
            },
        )
    });
}

criterion_group!(benches, tree_ops, tree_iter_down, tree_iter_up);
criterion_main!(benches);
