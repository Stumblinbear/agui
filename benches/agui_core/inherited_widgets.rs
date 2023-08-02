use criterion::{criterion_group, criterion_main, Criterion};

use agui::{
    manager::WidgetManager,
    prelude::{InheritedWidget, Widget},
};

#[derive(InheritedWidget, Default)]
struct TestWidget {
    #[child]
    child: Option<Widget>,
}

impl InheritedWidget for TestWidget {
    fn should_notify(&self, _: &Self) -> bool {
        true
    }
}

fn inherited_widgets(c: &mut Criterion) {
    let mut group = c.benchmark_group("widget manager");

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

criterion_group!(benches, inherited_widgets);
criterion_main!(benches);
