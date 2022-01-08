use criterion::{black_box, criterion_group, criterion_main, Criterion};

use agui::context::tree::Tree;

fn tree_ops(c: &mut Criterion) {
    c.bench_function("add to tree", |b| {
        b.iter_with_setup(
            || black_box(Tree::<usize>::default()),
            |mut tree| {
                tree.add(None, 0);
            },
        )
    });
    
    c.bench_function("remove from tree", |b| {
        b.iter_with_setup(
            || {
                let mut tree = black_box(Tree::<usize>::default());

                tree.add(None, 0);

                tree
            },
            |mut tree| {
                tree.remove(&0);
            },
        )
    });
}

fn get_deepest_child(c: &mut Criterion) {
    c.bench_function("get deepest child", |b| {
        b.iter_with_setup(
            || {
                let mut tree = black_box(Tree::<usize>::default());

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

fn iter_down(c: &mut Criterion) {
    c.bench_function("iterate down from root", |b| {
        b.iter_with_setup(
            || {
                let mut tree = black_box(Tree::<usize>::default());

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);

                tree
            },
            |tree| {
                let mut walker = tree.iter();

                while walker.next().is_some() { }
            },
        )
    });

    c.bench_function("iterate down from child", |b| {
        b.iter_with_setup(
            || {
                let mut tree = black_box(Tree::<usize>::default());

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);
                tree.add(Some(3), 4);

                tree
            },
            |tree| {
                let mut walker = tree.iter_from(1);

                while walker.next().is_some() { }
            },
        )
    });
}

fn iter_up(c: &mut Criterion) {
    c.bench_function("iterate up parents", |b| {
        b.iter_with_setup(
            || {
                let mut tree = black_box(Tree::<usize>::default());

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);

                tree
            },
            |tree| {
                let mut walker = tree.iter_parents(3);

                while walker.next().is_some() { }
            },
        )
    });

    c.bench_function("iterate up", |b| {
        b.iter_with_setup(
            || {
                let mut tree = black_box(Tree::<usize>::default());

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);

                tree
            },
            |tree| {
                let mut walker = tree.iter_up();

                while walker.next().is_some() { }
            },
        )
    });
    
    c.bench_function("iterate up from child", |b| {
        b.iter_with_setup(
            || {
                let mut tree = black_box(Tree::<usize>::default());

                tree.add(None, 0);
                tree.add(Some(0), 1);
                tree.add(Some(1), 2);
                tree.add(Some(2), 3);
                tree.add(Some(3), 4);

                tree
            },
            |tree| {
                let mut walker = tree.iter_up_from(3);

                while walker.next().is_some() { }
            },
        )
    });
}

criterion_group!(benches, tree_ops, get_deepest_child, iter_down, iter_up);
criterion_main!(benches);
