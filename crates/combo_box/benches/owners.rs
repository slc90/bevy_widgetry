use bevy::input::{mouse::MouseScrollUnit, touch::TouchPhase};
use bevy::picking::events::{Pointer, PointerScroll};
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::ScrollPosition;
use bevy::ui_widgets::Activate;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::color::WidgetryStyleOwner;
use bevy_widgetry_list_view::{WidgetryListModel, WidgetryListView, WidgetryListViewRenderer};
use bevy_widgetry_scroll_area::WidgetryScrollAreaViewport;
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_test_utils::primary_click;
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Resource, Default)]
struct HoverRequest(Option<(Entity, bool)>);

fn apply_hover(world: &mut World) -> Result {
    // Picking owns Hovered, so apply this style stimulus after its projection.
    if let Some((entity, hovered)) = world.resource_mut::<HoverRequest>().0.take() {
        world
            .get_entity_mut(entity)
            .map_err(BevyError::error)?
            .insert(Hovered(hovered));
    }
    Ok(())
}

struct Fixture {
    app: App,
    roots: Vec<Entity>,
    field: Entity,
    list: Entity,
    count: Option<u32>,
    hovered: bool,
    viewport: Entity,
    scroll: bool,
}

fn fixture(count: usize, action: &str) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.register_widgetry_combo_box::<String>()?
        .init_resource::<HoverRequest>()
        .add_systems(Update, apply_hover);
    let mut model = WidgetryListModel::default();
    for index in 0..100 {
        model.push(format!("Item {index}"))?;
    }
    let source = app.world_mut().spawn(model).id();
    let mut roots = Vec::new();
    for _ in 0..count {
        roots.push(
            app.world_mut()
                .spawn_scene(bsn! {
                    @WidgetryComboBox::<String> {
                        @source: source,
                        @renderer: {WidgetryListViewRenderer::new(|_, value: &String| bsn_list! {Text({value.clone()}) bevy_widgetry_core::text::WidgetryText})},
                    }
                })
                .map_err(BevyError::error)?
                .id(),
        );
    }
    settle(&mut app);
    let children = app
        .world()
        .get::<Children>(roots[0])
        .ok_or_else(|| missing("ComboBox children"))?;
    let field = children[0];
    let popup = children[1];
    let list = app
        .world()
        .get::<Children>(popup)
        .ok_or_else(|| missing("Popup children"))?[0];
    if app
        .world()
        .get::<ComputedNode>(field)
        .is_none_or(|node| node.size().min_element() <= 0.0)
    {
        return Err(missing("laid out Field"));
    }
    if action == "scroll_one" {
        app.world_mut().trigger(Activate { entity: field });
        settle(&mut app);
    }
    let viewport = app
        .world()
        .get::<Children>(list)
        .ok_or_else(|| missing("List children"))?
        .iter()
        .find(|entity| {
            app.world()
                .get::<WidgetryScrollAreaViewport>(*entity)
                .is_some()
        })
        .ok_or_else(|| missing("List viewport"))?;
    validate_text(&mut app)?;
    Ok(Fixture {
        app,
        roots,
        field,
        list,
        count: None,
        hovered: false,
        viewport,
        scroll: action == "scroll_one",
    })
}

fn validate(fixture: &mut Fixture) -> Result<u32> {
    let world = fixture.app.world();
    for (index, &root) in fixture.roots.iter().enumerate() {
        let children = world
            .get::<Children>(root)
            .ok_or_else(|| missing("ComboBox children"))?;
        let field = children[0];
        let popup = children[1];
        let list = world
            .get::<Children>(popup)
            .ok_or_else(|| missing("Popup children"))?[0];
        let colors = WidgetryThemeMode::Dark.colors().combo_box;
        let expected = if index == 0 && fixture.scroll {
            colors.field.open.background
        } else if index == 0 && fixture.hovered {
            colors.field.hovered.background
        } else {
            colors.field.normal.background
        };
        if world
            .get::<WidgetryStyleOwner<WidgetryButton>>(field)
            .is_none_or(|owner| owner.entity != root)
            || world
                .get::<WidgetryStyleOwner<WidgetryListView<String>>>(list)
                .is_none_or(|owner| owner.entity != root)
            || world.get::<BackgroundColor>(field).map(|color| color.0) != Some(expected)
        {
            return Err(missing("current ComboBox ownership and Field color"));
        }
    }
    if fixture.scroll {
        let rows = world
            .get::<Children>(fixture.viewport)
            .ok_or_else(|| missing("viewport content"))?[0];
        let rows = world
            .get::<Children>(rows)
            .ok_or_else(|| missing("visible rows"))?
            .iter()
            .filter(|entity| {
                world
                    .get::<bevy_widgetry_list_view::WidgetryListViewItem>(*entity)
                    .is_some()
            })
            .collect::<Vec<_>>();
        let start = (world
            .get::<ScrollPosition>(fixture.viewport)
            .ok_or_else(|| missing("scroll position"))?
            .0
            .y
            / 32.0) as usize;
        if rows.len() != 8 {
            return Err(missing(&format!(
                "eight rows follow popup scroll: actual={}, viewport={:?}, position={:?}",
                rows.len(),
                world
                    .get::<ComputedNode>(fixture.viewport)
                    .map(ComputedNode::size),
                world.get::<ScrollPosition>(fixture.viewport)
            )));
        }
        let colors = WidgetryThemeMode::Dark.colors().combo_box.list.item;
        for (offset, row) in rows.into_iter().enumerate() {
            let item = world
                .get::<bevy_widgetry_list_view::WidgetryListViewItem>(row)
                .ok_or_else(|| missing("visible row item"))?;
            let expected = if item.index == 0 {
                colors.selected
            } else {
                colors.normal
            };
            let text = world
                .get::<Children>(row)
                .ok_or_else(|| missing("row text"))?[0];
            if item.index != start + offset
                || world.get::<BackgroundColor>(row).map(|color| color.0)
                    != Some(expected.background)
                || world.get::<TextColor>(text).map(|color| color.0) != Some(expected.foreground)
            {
                return Err(missing("reused row identity and owner colors"));
            }
        }
    }

    let count = world.entities().len();
    if fixture.count.is_some_and(|previous| previous != count) {
        return Err(missing("stable entity count"));
    }
    fixture.count = Some(count);
    Ok(count)
}

fn main() -> Result {
    let mut harness = Harness::new("combo-box-owners-criterion")?;
    for count in [10, 100, 1000] {
        for action in ["idle", "repair_one", "hover_one", "scroll_one"] {
            run(
                &mut harness,
                &format!("owners/n{count}/{action}"),
                false,
                || fixture(count, action),
                |fixture, _| {
                    if action == "repair_one" {
                        fixture
                            .app
                            .world_mut()
                            .entity_mut(fixture.field)
                            .remove::<WidgetryStyleOwner<WidgetryButton>>();
                        fixture
                            .app
                            .world_mut()
                            .entity_mut(fixture.list)
                            .remove::<WidgetryStyleOwner<WidgetryListView<String>>>();
                    }
                    if action == "hover_one" {
                        fixture.hovered = !fixture.hovered;
                        fixture.app.world_mut().resource_mut::<HoverRequest>().0 =
                            Some((fixture.field, fixture.hovered));
                    }
                    if fixture.scroll {
                        let position = fixture
                            .app
                            .world()
                            .get::<ScrollPosition>(fixture.viewport)
                            .ok_or_else(|| missing("scroll position"))?
                            .0
                            .y;
                        let pointer = primary_click(fixture.viewport);
                        fixture.app.world_mut().trigger(PointerScroll {
                            entity: fixture.viewport,
                            pointer: Pointer::new(pointer.pointer.id, pointer.pointer.location()),
                            x: 0.0,
                            y: if position == 0.0 { -64.0 } else { 64.0 },
                            unit: MouseScrollUnit::Pixel,
                            hit: pointer.hit,
                            phase: TouchPhase::Moved,
                        });
                        fixture.app.world_mut().flush();
                        let next = fixture
                            .app
                            .world()
                            .get::<ScrollPosition>(fixture.viewport)
                            .ok_or_else(|| missing("scroll result"))?
                            .0
                            .y;
                        if next != if position == 0.0 { 64.0 } else { 0.0 } {
                            return Err(missing("popup scroll must alternate between 0 and 64"));
                        }
                    }
                    fixture.app.update();
                    Ok(())
                },
                validate,
            )?;
        }
    }
    harness.finish()
}
