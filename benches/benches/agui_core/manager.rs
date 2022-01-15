use criterion::{criterion_group, criterion_main, Criterion};

use agui::{
    macros::build,
    widget::WidgetRef,
    widgets::primitives::{Column, Drawable},
    WidgetManager,
};

fn widget_manager_ops(c: &mut Criterion) {
    c.bench_function("noop manager", |b| {
        b.iter_with_setup(
            || (Vec::new(), WidgetManager::default()),
            |(mut events, mut manager)| manager.update(&mut events),
        )
    });

    c.bench_function("add to manager", |b| {
        b.iter_with_setup(
            || {
                (
                    Vec::new(),
                    WidgetManager::default(),
                    WidgetRef::new(Drawable::default()),
                )
            },
            |(mut events, mut manager, widget)| {
                manager.add(None, widget);

                manager.update(&mut events);
            },
        )
    });

    c.bench_function("remove from manager", |b| {
        b.iter_with_setup(
            || {
                let mut manager = WidgetManager::default();

                let mut events = Vec::new();

                manager.add(None, WidgetRef::new(Drawable::default()));

                manager.update(&mut events);

                let root_id = manager
                    .get_context()
                    .get_tree()
                    .get_root()
                    .expect("failed to add widget");

                (Vec::new(), manager, root_id)
            },
            |(mut events, mut manager, widget_id)| {
                manager.remove(widget_id);

                manager.update(&mut events);
            },
        )
    });
}

fn widget_manager_nested_ops(c: &mut Criterion) {
    c.bench_function("add nested to manager", |b| {
        b.iter_with_setup(
            || {
                (
                    Vec::new(),
                    WidgetManager::default(),
                    WidgetRef::new::<Drawable>(build! {
                        Drawable {
                            child: Drawable {
                                child: Drawable {
                                    child: Drawable { },
                                },
                            },
                        }
                    }),
                )
            },
            |(mut events, mut manager, widget)| {
                manager.add(None, widget);

                manager.update(&mut events);
            },
        )
    });

    c.bench_function("remove nested from manager", |b| {
        b.iter_with_setup(
            || {
                let mut manager = WidgetManager::default();

                let mut events = Vec::new();

                manager.add(
                    None,
                    WidgetRef::new::<Drawable>(build! {
                        Drawable {
                            child: Drawable {
                                child: Drawable {
                                    child: Drawable { },
                                },
                            },
                        }
                    }),
                );

                manager.update(&mut events);

                let root_id = manager
                    .get_context()
                    .get_tree()
                    .get_root()
                    .expect("failed to add widget");

                (Vec::new(), manager, root_id)
            },
            |(mut events, mut manager, widget_id)| {
                manager.remove(widget_id);

                manager.update(&mut events);
            },
        )
    });
}

fn widget_manager_many_ops(c: &mut Criterion) {
    c.bench_function("add many to manager", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Drawable::default().into());
                }

                (Vec::new(), WidgetManager::default(), WidgetRef::new(column))
            },
            |(mut events, mut manager, widget)| {
                manager.add(None, widget);

                manager.update(&mut events);
            },
        )
    });

    c.bench_function("remove many from manager", |b| {
        b.iter_with_setup(
            || {
                let mut column = Column::default();

                for _ in 0..1000 {
                    column.children.push(Drawable::default().into());
                }

                let mut manager = WidgetManager::default();

                let mut events = Vec::new();

                manager.add(None, WidgetRef::new(column));

                manager.update(&mut events);

                let root_id = manager
                    .get_context()
                    .get_tree()
                    .get_root()
                    .expect("failed to add widget");

                (Vec::new(), manager, root_id)
            },
            |(mut events, mut manager, widget_id)| {
                manager.remove(widget_id);

                manager.update(&mut events);
            },
        )
    });
}

criterion_group!(
    benches,
    widget_manager_ops,
    widget_manager_nested_ops,
    widget_manager_many_ops
);
criterion_main!(benches);
