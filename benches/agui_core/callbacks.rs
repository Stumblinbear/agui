use criterion::{criterion_group, criterion_main, Criterion};

use agui::{
    manager::WidgetManager,
    prelude::{InheritedWidget, Widget},
};

fn callbacks(c: &mut Criterion) {
    let mut group = c.benchmark_group("callbacks");

    group.sample_size(500).bench_function("creation", |b| {
        b.iter_with_setup(
            || (WidgetManager::new(), TestWidget::default()),
            |(mut manager, widget)| {
                manager.set_root(widget);

                manager.update();
            },
        )
    });
}

criterion_group!(benches, callbacks);
criterion_main!(benches);
