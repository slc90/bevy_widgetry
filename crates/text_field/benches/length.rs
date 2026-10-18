#![allow(clippy::disallowed_macros, clippy::unwrap_used, clippy::expect_used)]

#[path = "../tests/support/mod.rs"]
mod support;

use bevy::{
    prelude::*,
    text::{EditableText, TextEdit},
};
use bevy_widgetry_text_field::{WidgetryTextField, WidgetryTextFieldPlugin};
use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};

fn length(c: &mut Criterion) {
    let mut app = support::editing_app();
    app.add_plugins(WidgetryTextFieldPlugin);
    let entity = app
        .world_mut()
        .spawn_scene(bsn! {
            @WidgetryTextField { @max_length: {Some(20)} }
        })
        .unwrap()
        .id();
    app.update();
    let mut group = c.benchmark_group("limited_edit_frame");
    group.warm_up_time(std::time::Duration::from_secs(1));
    group.measurement_time(std::time::Duration::from_secs(2));
    group.sample_size(20);
    for size in [100, 10_000, 100_000] {
        let input = "文".repeat(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &input, |b, input| {
            b.iter(|| {
                app.world_mut()
                    .get_mut::<EditableText>(entity)
                    .unwrap()
                    .pending_edits
                    .extend([TextEdit::SelectAll, TextEdit::Insert(input.as_str().into())]);
                app.update();
                std::hint::black_box(WidgetryTextField::text(app.world(), entity).unwrap());
            });
        });
        assert_eq!(
            WidgetryTextField::text(app.world(), entity).unwrap(),
            "文".repeat(20)
        );
    }
    group.finish();
}

criterion_group!(benches, length);
criterion_main!(benches);
