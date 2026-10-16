use bevy::prelude::*;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_combo_box::{WidgetryComboBox, WidgetryComboBoxAppExt};
use bevy_widgetry_core::color::WidgetryStyleOwner;
use bevy_widgetry_list_view::{WidgetryListModel, WidgetryListView, WidgetryListViewRenderer};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_theme::WidgetryThemeMode;

struct Fixture {
    app: App,
    roots: Vec<Entity>,
    field: Entity,
    list: Entity,
    count: Option<u32>,
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.register_widgetry_combo_box::<String>()?;
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
    validate_text(&mut app)?;
    Ok(Fixture {
        app,
        roots,
        field,
        list,
        count: None,
    })
}

fn validate(fixture: &mut Fixture) -> Result<u32> {
    let world = fixture.app.world();
    for &root in &fixture.roots {
        let children = world
            .get::<Children>(root)
            .ok_or_else(|| missing("ComboBox children"))?;
        let field = children[0];
        let popup = children[1];
        let list = world
            .get::<Children>(popup)
            .ok_or_else(|| missing("Popup children"))?[0];
        if world
            .get::<WidgetryStyleOwner<WidgetryButton>>(field)
            .is_none_or(|owner| owner.entity != root)
            || world
                .get::<WidgetryStyleOwner<WidgetryListView<String>>>(list)
                .is_none_or(|owner| owner.entity != root)
            || world.get::<BackgroundColor>(field).map(|color| color.0)
                != Some(
                    WidgetryThemeMode::Dark
                        .colors()
                        .combo_box
                        .field
                        .normal
                        .background,
                )
        {
            return Err(missing("current ComboBox ownership and Field color"));
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
        for action in ["idle", "repair_one"] {
            run(
                &mut harness,
                &format!("owners/n{count}/{action}"),
                false,
                || fixture(count),
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
                    fixture.app.update();
                    Ok(())
                },
                validate,
            )?;
        }
    }
    harness.finish()
}
