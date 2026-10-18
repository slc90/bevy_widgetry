use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
use bevy::picking::events::{Pointer, PointerScroll};
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::Activate;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt, WidgetryComboBoxMatcher};
use bevy_widgetry_list_view::{
    WidgetryListModel, WidgetryListProjection, WidgetryListViewItem, WidgetryListViewRenderer,
};
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_test_utils::primary_click;
use bevy_widgetry_text_field::WidgetryTextField;

struct Fixture {
    app: App,
    input: Entity,
    list: Entity,
    viewport: Entity,
    counts: [usize; 4],
    expected: usize,
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.register_widgetry_combo_box::<String>()?;
    let mut model = WidgetryListModel::default();
    for i in 0..count {
        model.push(format!("Item {i:05}"))?;
    }
    let source = app.world_mut().spawn(model).id();
    let root = app.world_mut().spawn_scene(bsn! {
        @WidgetryComboBox::<String> {
            @source: source, @max_visible_items: 8,
            @search: {Some(WidgetryComboBoxMatcher::new(|query: &str, item: &String| item.contains(query)))},
            @renderer: {WidgetryListViewRenderer::new(|item: &String| bsn_list!{Text({item.clone()}) bevy_widgetry_core::text::WidgetryText})},
        }
        Node {width: px(300)}
    }).map_err(BevyError::error)?.id();
    settle(&mut app);
    let children = app
        .world()
        .get::<Children>(root)
        .ok_or_else(|| missing("ComboBox parts"))?;
    let (field, popup) = (children[0], children[1]);
    let children = app
        .world()
        .get::<Children>(popup)
        .ok_or_else(|| missing("search parts"))?;
    let (input, list) = (children[0], children[1]);
    app.world_mut().trigger(Activate { entity: field });
    settle(&mut app);
    let viewport = app
        .world()
        .get::<Children>(list)
        .and_then(|children| {
            children.iter().find(|child| {
                app.world()
                    .get::<WidgetryScrollAreaViewport>(*child)
                    .is_some()
            })
        })
        .ok_or_else(|| missing("viewport"))?;
    let counts = [
        1,
        count,
        0,
        (0..count)
            .filter(|i| format!("Item {i:05}").contains("99"))
            .count(),
    ];
    Ok(Fixture {
        app,
        input,
        list,
        viewport,
        counts,
        expected: count,
    })
}

fn validate(f: &mut Fixture) -> Result<u32> {
    if f.app
        .world()
        .get::<WidgetryListProjection>(f.list)
        .is_none_or(|projection| projection.len() != f.expected)
    {
        return Err(missing("expected search result count"));
    }
    let rows = f
        .app
        .world_mut()
        .query::<&WidgetryListViewItem>()
        .iter(f.app.world())
        .count();
    if rows > 10 || (f.expected > 0 && rows == 0) {
        return Err(missing("bounded rendered result window"));
    }
    validate_text(&mut f.app)?;
    Ok(f.app.world().entities().len())
}

fn main() -> Result {
    let mut harness = Harness::new("combo-box-search-criterion")?;
    for count in [1000, 10_000] {
        run(
            &mut harness,
            &format!("search/n{count}/query"),
            false,
            || fixture(count),
            |f, i| {
                let query = ["00001", "Item", "missing", "99"][i % 4];
                f.expected = f.counts[i % 4];
                WidgetryTextField::set_text_in_world(f.app.world_mut(), f.input, query)?;
                f.app.update();
                // A result-count change alters popup geometry, consumed by virtualization on the next frame.
                f.app.update();
                Ok(())
            },
            validate,
        )?;
    }
    run(
        &mut harness,
        "search/n10000/result_window",
        false,
        || fixture(10_000),
        |f, _| {
            let current = f
                .app
                .world()
                .get::<ScrollPosition>(f.viewport)
                .ok_or_else(|| missing("scroll position"))?
                .0
                .y;
            let click = primary_click(f.viewport);
            f.app.world_mut().trigger(PointerScroll {
                entity: f.viewport,
                pointer: Pointer::new(click.pointer.id, click.pointer.location()),
                x: 0.0,
                y: if current == 0.0 { -640.0 } else { 640.0 },
                unit: MouseScrollUnit::Pixel,
                phase: TouchPhase::Moved,
                hit: click.hit,
            });
            f.app.world_mut().flush();
            f.app.update();
            let next = f
                .app
                .world()
                .get::<ScrollPosition>(f.viewport)
                .ok_or_else(|| missing("scroll result"))?
                .0
                .y;
            if next != if current == 0.0 { 640.0 } else { 0.0 } {
                return Err(missing("result window scroll stimulus"));
            }
            Ok(())
        },
        validate,
    )?;
    harness.finish()
}
