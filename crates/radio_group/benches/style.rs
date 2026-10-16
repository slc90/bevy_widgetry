use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy_widgetry_core::foreground::ResolvedForeground;
use bevy_widgetry_radio_group::{
    WidgetryRadioGroup, WidgetryRadioGroupColorOverrides, WidgetryRadioGroupPlugin,
    WidgetryRadioOption,
};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app};
use bevy_widgetry_theme::WidgetryThemeMode;

#[derive(Resource, Default)]
struct HoverRequest(Option<(Entity, bool)>);

struct Fixture {
    app: App,
    root: Entity,
    option: Entity,
    indicator: Entity,
    border: Color,
    foreground: Color,
}

fn apply_hover(world: &mut World) -> Result {
    // Picking runs before this stimulus so it cannot reset the requested benchmark input.
    if let Some((entity, hovered)) = world.resource_mut::<HoverRequest>().0.take() {
        world
            .get_entity_mut(entity)
            .map_err(BevyError::error)?
            .insert(Hovered(hovered));
    }
    Ok(())
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins(WidgetryRadioGroupPlugin)
        .init_resource::<HoverRequest>()
        .add_systems(Update, apply_hover);
    let mut roots = Vec::new();
    for _ in 0..count {
        roots.push(
            app.world_mut()
                .spawn_scene(bsn! {
                    @WidgetryRadioGroup Children [
                        @WidgetryRadioOption-- @WidgetryRadioOption-- @WidgetryRadioOption
                    ]
                })
                .map_err(BevyError::error)?
                .id(),
        );
    }
    settle(&mut app);
    let root = roots[0];
    let option = app
        .world()
        .get::<Children>(root)
        .ok_or_else(|| missing("Options"))?[1];
    let indicator = app
        .world()
        .get::<Children>(option)
        .ok_or_else(|| missing("Indicator"))?[0];
    if app
        .world()
        .get::<ComputedNode>(indicator)
        .is_none_or(|node| node.size().min_element() <= 0.0)
    {
        return Err(missing("laid out Indicator"));
    }
    let colors = WidgetryThemeMode::Dark
        .colors()
        .radio_group
        .option
        .unchecked
        .normal;
    Ok(Fixture {
        app,
        root,
        option,
        indicator,
        border: colors.border,
        foreground: colors.foreground,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("radio-option-style-criterion")?;
    for count in [10, 100, 1000] {
        for action in ["idle", "hover_one", "select_one", "override_group"] {
            run(
                &mut harness,
                &format!("style/n{count}/{action}"),
                false,
                || fixture(count),
                |fixture, index| {
                    let active = index.is_multiple_of(2);
                    let colors = &WidgetryThemeMode::Dark.colors().radio_group.option;
                    match action {
                        "hover_one" => {
                            fixture.app.world_mut().resource_mut::<HoverRequest>().0 =
                                Some((fixture.option, active));
                            fixture.border = if active {
                                colors.unchecked.hovered.border
                            } else {
                                colors.unchecked.normal.border
                            };
                        }
                        "select_one" => {
                            WidgetryRadioGroup::set_selected(
                                &mut fixture.app.world_mut().commands(),
                                fixture.root,
                                usize::from(active),
                            );
                            fixture.border = if active {
                                colors.checked.normal.border
                            } else {
                                colors.unchecked.normal.border
                            };
                        }
                        "override_group" => {
                            let mut overrides = WidgetryRadioGroupColorOverrides::default();
                            fixture.foreground = if active { Color::BLACK } else { Color::WHITE };
                            overrides.option.unchecked.normal.foreground = Some(fixture.foreground);
                            WidgetryRadioGroupColorOverrides::set_in_world(
                                fixture.app.world_mut(),
                                fixture.root,
                                overrides,
                            )?;
                        }
                        _ => {}
                    }
                    // Includes the real headless App update and UI layout, excluding GPU and native presentation.
                    fixture.app.update();
                    Ok(())
                },
                |fixture| {
                    let world = fixture.app.world();
                    if world.get::<BorderColor>(fixture.indicator)
                        != Some(&BorderColor::all(fixture.border))
                        || world.get::<ResolvedForeground>(fixture.option)
                            != Some(&ResolvedForeground(fixture.foreground))
                    {
                        return Err(missing("expected Option colors"));
                    }
                    Ok(world.entities().len())
                },
            )?;
        }
    }
    harness.finish()
}
