use bevy::camera::RenderTargetInfo;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy_widgetry_button::WidgetryButton;
use bevy_widgetry_core::color::WidgetryStyleOwner;
use bevy_widgetry_message_box::{
    WidgetryMessageBoxButtons, WidgetryMessageBoxPlugin, widgetry_message_box,
};
use bevy_widgetry_test_utils::benchmark::{Harness, missing, run, settle, ui_app, validate_text};
use bevy_widgetry_window::{
    WidgetryWindowBackground, WidgetryWindowControlsConfig, internal::WindowRoot,
    owned_widgetry_window, widgetry_window_target,
};

#[derive(Component)]
struct MoveSlot(usize, usize);

#[derive(Component)]
struct BusinessButton;

#[derive(Resource, Default)]
struct InputRequest(Option<(Entity, bool, bool)>);

fn apply_input(world: &mut World) -> Result {
    // Picking projects Hovered before Update; inject the style stimulus afterwards.
    if let Some((entity, pressed, active)) = world.resource_mut::<InputRequest>().0.take() {
        let mut entity = world.get_entity_mut(entity).map_err(BevyError::error)?;
        if pressed {
            if active {
                entity.insert(Pressed);
            } else {
                entity.remove::<Pressed>();
            }
        } else {
            entity.insert(Hovered(active));
        }
    }
    Ok(())
}

struct Fixture {
    app: App,
    roots: Vec<Entity>,
    actions: Vec<(Entity, Entity)>,
    slots: [Entity; 2],
    moving: Entity,
}

fn fixture(count: usize) -> Result<Fixture> {
    let mut app = ui_app()?;
    app.add_plugins(WidgetryMessageBoxPlugin)
        .init_resource::<InputRequest>()
        .add_systems(Update, apply_input);
    let parent = app
        .world_mut()
        .spawn_scene(bsn! {
            @owned_widgetry_window(Window::default(), WidgetryWindowControlsConfig::default(),
                WidgetryWindowBackground::Theme, Default::default(), bsn_list!{}, bsn_list!{})
        })
        .map_err(BevyError::error)?
        .id();
    let native = widgetry_window_target(app.world(), parent)
        .ok_or_else(|| missing("parent native Window"))?;
    app.world_mut()
        .entity_mut(native)
        .insert(bevy::window::PrimaryWindow);
    let mut roots = Vec::new();
    for index in 0..count {
        let business = (0..8)
            .map(|_| {
                bsn! {
                    @WidgetryButton { @children: {Some(Box::new(bsn_list! {Text("Business content") bevy_widgetry_core::text::WidgetryText}) as Box<dyn bevy::prelude::SceneList>)}, } template(|_| Ok(BusinessButton))

                }
            })
            .collect::<Vec<_>>();
        roots.push(app.world_mut().spawn_scene(bsn! {
            @widgetry_message_box(native, "Owner benchmark", WidgetryMessageBoxButtons::YesNoCancel,
                Default::default(), bsn_list! {
                    Node {flex_direction: FlexDirection::Column} Children [
                        Node template(move |_| Ok(MoveSlot(index, 0)))--
                        Node template(move |_| Ok(MoveSlot(index, 1)))--
                        Node {flex_direction: FlexDirection::Column} Children [{business}]
                    ]
                })
        }).map_err(BevyError::error)?.id());
    }
    // Headless UI layout needs the same target dimensions a live camera would receive.
    let cameras = app
        .world_mut()
        .query::<(&WindowRoot, &UiTargetCamera)>()
        .iter(app.world())
        .map(|(root, camera)| (root.target_window, camera.0))
        .collect::<Vec<_>>();
    for (native, camera) in cameras {
        let window = app
            .world()
            .get::<Window>(native)
            .ok_or_else(|| missing("native Window"))?;
        let info = RenderTargetInfo {
            physical_size: window.resolution.physical_size(),
            scale_factor: window.resolution.scale_factor(),
        };
        app.world_mut()
            .get_mut::<Camera>(camera)
            .ok_or_else(|| missing("Camera"))?
            .computed
            .target_info = Some(info);
    }
    settle(&mut app);
    let actions = app
        .world_mut()
        .query::<(Entity, &WidgetryStyleOwner<WidgetryButton>)>()
        .iter(app.world())
        .map(|(entity, owner)| (entity, owner.entity))
        .collect::<Vec<_>>();
    if actions.len() != count * 3 {
        return Err(missing("three owned result buttons per dialog"));
    }
    let moving = actions
        .iter()
        .find(|(_, root)| *root == roots[0])
        .ok_or_else(|| missing("first result button"))?
        .0;
    let mut slots = [Entity::PLACEHOLDER; 2];
    for (entity, slot) in app
        .world_mut()
        .query::<(Entity, &MoveSlot)>()
        .iter(app.world())
    {
        if slot.0 == 0 {
            slots[slot.1] = entity;
        }
    }
    if slots.contains(&Entity::PLACEHOLDER) {
        return Err(missing("move slots"));
    }
    app.world_mut().entity_mut(moving).insert(ChildOf(slots[1]));
    settle(&mut app);
    for &root in &roots {
        if app
            .world()
            .get::<ComputedNode>(root)
            .is_none_or(|node| node.size().min_element() <= 0.0)
        {
            return Err(missing("laid out MessageBox"));
        }
    }
    validate_text(&mut app)?;
    Ok(Fixture {
        app,
        roots,
        actions,
        slots,
        moving,
    })
}

fn main() -> Result {
    let mut harness = Harness::new("message-box-owners")?;
    for count in [1, 12, 100] {
        for action in ["idle", "reparent_one", "hover_one", "press_one"] {
            run(
                &mut harness,
                &format!("message-box-owners/n{count}/{action}"),
                false,
                || fixture(count),
                |fixture, index| {
                    if action == "reparent_one" {
                        fixture
                            .app
                            .world_mut()
                            .entity_mut(fixture.moving)
                            .insert(ChildOf(fixture.slots[index % 2]));
                    }
                    if action == "hover_one" {
                        fixture.app.world_mut().resource_mut::<InputRequest>().0 =
                            Some((fixture.moving, false, index.is_multiple_of(2)));
                    }
                    if action == "press_one" {
                        fixture.app.world_mut().resource_mut::<InputRequest>().0 =
                            Some((fixture.moving, true, index.is_multiple_of(2)));
                    }
                    fixture.app.update();
                    if action == "hover_one" || action == "press_one" {
                        let colors = fixture
                            .app
                            .world()
                            .resource::<bevy_widgetry_theme::WidgetryThemeMode>()
                            .colors()
                            .message_box
                            .action_button;
                        let expected = if fixture
                            .app
                            .world()
                            .get::<bevy::ui::InteractionDisabled>(fixture.moving)
                            .is_some()
                        {
                            colors.disabled
                        } else if index % 2 != 0 {
                            colors.normal
                        } else if action == "press_one" {
                            colors.pressed
                        } else {
                            colors.hovered
                        };
                        if fixture.app.world().get::<BackgroundColor>(fixture.moving)
                            != Some(&BackgroundColor(expected.background))
                        {
                            return Err(BevyError::error(
                                "MessageBox action color invariant failed",
                            ));
                        }
                    }
                    for &root in &fixture.roots {
                        if fixture
                            .app
                            .world()
                            .get::<WidgetryStyleOwner<WindowRoot>>(root)
                            .is_none_or(|owner| owner.entity != root)
                        {
                            return Err(BevyError::error(
                                "MessageBox Window owner invariant failed",
                            ));
                        }
                    }
                    for &(entity, root) in &fixture.actions {
                        if fixture
                            .app
                            .world()
                            .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                            .is_none_or(|owner| owner.entity != root)
                        {
                            return Err(BevyError::error(
                                "MessageBox action owner invariant failed",
                            ));
                        }
                    }
                    for entity in fixture
                        .app
                        .world_mut()
                        .query_filtered::<Entity, With<BusinessButton>>()
                        .iter(fixture.app.world())
                    {
                        if fixture
                            .app
                            .world()
                            .get::<WidgetryStyleOwner<WidgetryButton>>(entity)
                            .is_some()
                        {
                            return Err(BevyError::error("Business Button was incorrectly owned"));
                        }
                    }
                    Ok(())
                },
                |fixture| Ok(fixture.app.world().entities().len()),
            )?;
        }
    }
    harness.finish()
}
