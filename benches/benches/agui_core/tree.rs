use criterion::{black_box, criterion_group, criterion_main, Criterion};

use agui::{tree::Tree, widget::WidgetId};

fn tree_ops(c: &mut Criterion) {
    c.bench_function("add to tree", |b| {
        b.iter_with_setup(Tree::<WidgetId, usize>::default, |mut tree| {
            tree.add(None, 0);
        })
    });

    c.bench_function("add many to tree", |b| {
        b.iter_with_setup(Tree::<WidgetId, usize>::default, |mut tree| {
            let root_id = tree.add(None, 0);

            for i in 0..1000 {
                tree.add(Some(root_id), i);
            }
        })
    });

    c.bench_function("remove from tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

                let widget_id = tree.add(None, 0);

                (tree, widget_id)
            },
            |(mut tree, widget_id)| {
                tree.remove(widget_id);
            },
        )
    });

    c.bench_function("remove many from tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

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
                    tree.remove(widget_id);
                }
            },
        )
    });
}

fn tree_get_deepest_child(c: &mut Criterion) {
    c.bench_function("get deepest child in tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

                let root_id = tree.add(None, 0);
                let parent_id = tree.add(Some(root_id), 1);
                let parent_id = tree.add(Some(parent_id), 2);
                tree.add(Some(parent_id), 3);

                (tree, root_id)
            },
            |(tree, root_id)| {
                black_box(tree.get_deepest_child(Some(root_id)));
            },
        )
    });
}

fn tree_iter_down(c: &mut Criterion) {
    c.bench_function("iterate down from tree root", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

                let parent_id = tree.add(None, 0);
                let parent_id = tree.add(Some(parent_id), 1);
                let parent_id = tree.add(Some(parent_id), 2);
                tree.add(Some(parent_id), 3);

                tree
            },
            |tree| {
                let mut walker = black_box(tree.iter());

                while walker.next().is_some() {}
            },
        )
    });

    c.bench_function("iterate down from tree child", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

                let parent_id = tree.add(None, 0);
                let widget_id = tree.add(Some(parent_id), 1);
                let parent_id = tree.add(Some(widget_id), 2);
                let parent_id = tree.add(Some(parent_id), 3);
                tree.add(Some(parent_id), 4);

                (tree, widget_id)
            },
            |(tree, widget_id)| {
                let mut walker = black_box(tree.iter_from(widget_id));

                while walker.next().is_some() {}
            },
        )
    });
}

fn tree_iter_up(c: &mut Criterion) {
    c.bench_function("iterate up tree parents", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

                let parent_id = tree.add(None, 0);
                let widget_id = tree.add(Some(parent_id), 1);
                let parent_id = tree.add(Some(widget_id), 2);
                let widget_id = tree.add(Some(parent_id), 3);

                (tree, widget_id)
            },
            |(tree, widget_id)| {
                let mut walker = black_box(tree.iter_parents(widget_id));

                while walker.next().is_some() {}
            },
        )
    });

    c.bench_function("iterate up tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

                let parent_id = tree.add(None, 0);
                let widget_id = tree.add(Some(parent_id), 1);
                let parent_id = tree.add(Some(widget_id), 2);
                tree.add(Some(parent_id), 3);

                tree
            },
            |tree| {
                let mut walker = black_box(tree.iter_up());

                while walker.next().is_some() {}
            },
        )
    });

    c.bench_function("iterate up from tree child", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<WidgetId, usize>::default();

                let parent_id = tree.add(None, 0);
                let widget_id = tree.add(Some(parent_id), 1);
                let parent_id = tree.add(Some(widget_id), 2);
                let widget_id = tree.add(Some(parent_id), 3);

                (tree, widget_id)
            },
            |(tree, widget_id)| {
                let mut walker = black_box(tree.iter_up_from(widget_id));

                while walker.next().is_some() {}
            },
        )
    });
}

criterion_group!(
    benches,
    tree_ops,
    tree_get_deepest_child,
    tree_iter_down,
    tree_iter_up
);
criterion_main!(benches);
