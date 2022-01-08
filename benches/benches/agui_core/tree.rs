use criterion::{black_box, criterion_group, criterion_main, Criterion};

use agui::context::tree::Tree;

fn tree_ops(c: &mut Criterion) {
    c.bench_function("add to tree", |b| {
        b.iter_with_setup(
            Tree::<usize>::default,
            |mut tree| {
                tree.add(None, 0);
            },
        )
    });
    
    c.bench_function("remove from tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<usize>::default();

                tree.add(None, 0);

                tree
            },
            |mut tree| {
                tree.remove(&0);
            },
        )
    });
}

fn tree_get_deepest_child(c: &mut Criterion) {
    c.bench_function("get deepest child in tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<usize>::default();

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);

                tree
            },
            |tree| {
                black_box(tree.get_deepest_child(Some(0)));
            },
        )
    });
}

fn tree_iter_down(c: &mut Criterion) {
    c.bench_function("iterate down from tree root", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<usize>::default();

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);

                tree
            },
            |tree| {
                let mut walker = black_box(tree.iter());

                while walker.next().is_some() { }
            },
        )
    });

    c.bench_function("iterate down from tree child", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<usize>::default();

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);
                tree.add(Some(3), 4);

                tree
            },
            |tree| {
                let mut walker = black_box(tree.iter_from(1));

                while walker.next().is_some() { }
            },
        )
    });
}

fn tree_iter_up(c: &mut Criterion) {
    c.bench_function("iterate up tree parents", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<usize>::default();

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);

                tree
            },
            |tree| {
                let mut walker = black_box(tree.iter_parents(3));

                while walker.next().is_some() { }
            },
        )
    });

    c.bench_function("iterate up tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<usize>::default();

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);

                tree
            },
            |tree| {
                let mut walker = black_box(tree.iter_up());

                while walker.next().is_some() { }
            },
        )
    });
    
    c.bench_function("iterate up from tree child", |b| {
        b.iter_with_setup(
            || {
                let mut tree = Tree::<usize>::default();

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);
                tree.add(Some(3), 4);

                tree
            },
            |tree| {
                let mut walker = black_box(tree.iter_up_from(3));

                while walker.next().is_some() { }
            },
        )
    });
}

criterion_group!(benches, tree_ops, tree_get_deepest_child, tree_iter_down, tree_iter_up);
criterion_main!(benches);
